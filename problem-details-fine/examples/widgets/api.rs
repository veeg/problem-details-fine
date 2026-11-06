//! The HTTP API of the widgets server: the domain, the extractors, the problem sets of the operations, the
//! handlers and the Openapi document. `main.rs` has the commands of the example.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{
        FromRequest, FromRequestParts, Path, Query, Request, State, rejection::PathRejection,
    },
    http::{StatusCode, header, request::Parts},
    routing::get,
};
use problem_details_fine::{
    embedded::{
        DependencyProblem, ForbiddenProblem, InternalServerErrorProblem, JsonType,
        MissingMimeProblem, MissingRequestBodyProblem, ParseMimeProblem, ParsePathParameterProblem,
        ParseQueryParameterProblem, ParseRequestBodyProblem, UnauthorizedProblem,
        UnhandledPathParameterErrorProblem, UnhandledQueryStringErrorProblem,
        UnhandledRequestBodyErrorProblem, UnknownPathOperationProblem, UnknownResourceProblem,
        UnsupportedMimeProblem,
    },
    problem_set, problem_type,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use utoipa::{OpenApi, ToSchema};

// The domain.

/// A widget.
#[derive(Debug, Clone, Serialize, ToSchema)]
struct Widget {
    /// The identifier of the widget.
    id: u64,
    /// The name of the widget. Each widget has a different name.
    name: String,
}

/// The body of a request that creates a widget.
#[derive(Debug, Deserialize, ToSchema)]
struct NewWidget {
    /// The name of the widget.
    name: String,
}

#[derive(Clone, Default)]
pub struct AppState {
    widgets: Arc<Mutex<BTreeMap<u64, Widget>>>,
}

/// A problem type of this service. The other problem types come from the `embedded` module.
#[problem_type(
    type = "https://widgets.example.com/problems/name-taken",
    status = CONFLICT,
    title = "A widget with this name already exists",
    detail = "the name `{name}` belongs to widget {existing_id}",
)]
#[derive(Debug)]
struct WidgetNameTaken {
    /// The name in the request.
    name: String,
    /// The identifier of the widget that has the name.
    existing_id: u64,
}

// Authentication.

/// The scopes of each bearer token that the server accepts.
const TOKENS: &[(&str, &[&str])] = &[
    ("reader-token", &["widgets:read"]),
    ("writer-token", &["widgets:read", "widgets:write"]),
];

/// The caller of the operation, from the `Authorization` header.
struct Caller {
    scopes: &'static [&'static str],
}

impl Caller {
    /// Returns `ForbiddenProblem` if the caller does not have `scope`.
    fn require(&self, scope: &str) -> Result<(), ForbiddenProblem> {
        if self.scopes.contains(&scope) {
            Ok(())
        } else {
            Err(ForbiddenProblem::new(&[scope]))
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Caller {
    type Rejection = UnauthorizedProblem;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, UnauthorizedProblem> {
        let value = parts
            .headers
            .get(header::AUTHORIZATION)
            .ok_or(UnauthorizedProblem::MissingAuthorization)?
            .to_str()
            .map_err(|_| UnauthorizedProblem::MalformedHeader)?;
        let (scheme, credentials) = value.split_once(' ').unwrap_or((value, ""));
        if !scheme.eq_ignore_ascii_case("bearer") {
            return Err(UnauthorizedProblem::unsupported_scheme(scheme, &["Bearer"]));
        }
        let token = credentials.trim();
        if token.is_empty() {
            return Err(UnauthorizedProblem::MissingCredentials);
        }
        TOKENS
            .iter()
            .find(|(known, _)| *known == token)
            .map(|(_, scopes)| Caller { scopes })
            .ok_or(UnauthorizedProblem::Credentials)
    }
}

// Path parameters.

/// The problems of the [`WidgetId`] extractor.
#[problem_set]
#[derive(Debug)]
enum PathError {
    Parse(ParsePathParameterProblem),
    Unhandled(UnhandledPathParameterErrorProblem),
}

/// The `{id}` path parameter.
struct WidgetId(u64);

impl<S: Send + Sync> FromRequestParts<S> for WidgetId {
    type Rejection = PathError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, PathError> {
        use axum::extract::path::ErrorKind;

        match Path::<u64>::from_request_parts(parts, state).await {
            Ok(Path(id)) => Ok(WidgetId(id)),
            Err(PathRejection::FailedToDeserializePathParams(error)) => {
                Err(match error.into_kind() {
                    ErrorKind::ParseErrorAtKey {
                        value,
                        expected_type,
                        ..
                    }
                    | ErrorKind::ParseErrorAtIndex {
                        value,
                        expected_type,
                        ..
                    }
                    | ErrorKind::ParseError {
                        value,
                        expected_type,
                    } => ParsePathParameterProblem::parse_error(value, expected_type).into(),
                    ErrorKind::DeserializeError {
                        key,
                        value,
                        message,
                    } => ParsePathParameterProblem::deserialize(key, value, message).into(),
                    // The decoded value is not available. The value in the problem is the segment as it is in the
                    // URI, for example `%FF`. `{id}` is the last segment of the path.
                    ErrorKind::InvalidUtf8InPathParam { key } => {
                        let raw = parts.uri.path().rsplit('/').next().unwrap_or_default();
                        ParsePathParameterProblem::deserialize(
                            key,
                            raw,
                            "the percent-decoded value is not valid UTF-8",
                        )
                        .into()
                    }
                    // The other kinds come from a fault in the server, for example a route without `{id}`.
                    _ => UnhandledPathParameterErrorProblem.into(),
                })
            }
            Err(_) => Err(UnhandledPathParameterErrorProblem.into()),
        }
    }
}

// Query parameters.

/// The problems of the [`Page`] extractor.
#[problem_set]
#[derive(Debug)]
enum QueryError {
    Parse(ParseQueryParameterProblem),
    Unhandled(UnhandledQueryStringErrorProblem),
}

/// The `limit` query parameter. The default is 20.
struct Page {
    limit: u32,
}

impl<S: Send + Sync> FromRequestParts<S> for Page {
    type Rejection = QueryError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, QueryError> {
        let Query(query) = Query::<HashMap<String, String>>::from_request_parts(parts, state)
            .await
            .map_err(|_| UnhandledQueryStringErrorProblem)?;
        let limit = match query.get("limit") {
            None => 20,
            Some(value) => value.parse().map_err(|error: std::num::ParseIntError| {
                ParseQueryParameterProblem::deserialize("limit", error, Some(value), Some("u32"))
            })?,
        };
        Ok(Page { limit })
    }
}

// The request body.

/// The problems of the [`JsonBody`] extractor.
#[problem_set]
#[derive(Debug)]
enum BodyError {
    MissingMime(MissingMimeProblem),
    UnsupportedMime(UnsupportedMimeProblem),
    ParseMime(ParseMimeProblem),
    MissingBody(MissingRequestBodyProblem),
    Parse(ParseRequestBodyProblem),
    Unhandled(UnhandledRequestBodyErrorProblem),
}

/// A JSON request body.
struct JsonBody<T>(T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequest<S> for JsonBody<T> {
    type Rejection = BodyError;

    async fn from_request(request: Request, state: &S) -> Result<Self, BodyError> {
        check_content_type(request.headers().get(header::CONTENT_TYPE))?;
        let bytes = Bytes::from_request(request, state)
            .await
            .map_err(|_| UnhandledRequestBodyErrorProblem)?;
        if bytes.is_empty() {
            return Err(MissingRequestBodyProblem.into());
        }
        let deserializer = &mut serde_json::Deserializer::from_slice(&bytes);
        serde_path_to_error::deserialize(deserializer)
            .map(JsonBody)
            .map_err(|error| parse_request_body(error).into())
    }
}

/// Accepts `application/json` and `application/<subtype>+json`, with or without parameters.
fn check_content_type(value: Option<&header::HeaderValue>) -> Result<(), BodyError> {
    let header = header::CONTENT_TYPE.as_str();
    let value = value.ok_or_else(|| MissingMimeProblem::new(header))?;
    let text = value
        .to_str()
        .map_err(|_| ParseMimeProblem::invalid_ascii(header, value.as_bytes().to_vec()))?;
    let essence = text
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let Some((kind, subtype)) = essence.split_once('/') else {
        return Err(ParseMimeProblem::parse_error(
            header,
            "a media type has the form `type/subtype`",
            text.into(),
        )
        .into());
    };
    if kind == "application" && (subtype == "json" || subtype.ends_with("+json")) {
        Ok(())
    } else {
        Err(UnsupportedMimeProblem::new(header, essence, &["application/json"]).into())
    }
}

/// Converts a JSON error into a [`ParseRequestBodyProblem`].
fn parse_request_body(
    error: serde_path_to_error::Error<serde_json::Error>,
) -> ParseRequestBodyProblem {
    use serde_json::error::Category;

    let path = error.path().to_string();
    let error = error.into_inner();
    let (line, column) = (error.line(), error.column());
    let message = error.to_string();
    match error.classify() {
        Category::Syntax | Category::Eof => {
            ParseRequestBodyProblem::syntax(path, message, line, column)
        }
        Category::Data => {
            // serde writes "missing field `name`" for a field that the JSON does not have.
            if let Some(field) = message
                .strip_prefix("missing field `")
                .and_then(|rest| rest.split('`').next())
            {
                let field = if path == "." {
                    field.to_owned()
                } else {
                    format!("{path}.{field}")
                };
                ParseRequestBodyProblem::missing_required_field(field)
            } else if let Some(unexpected) = message.strip_prefix("invalid type: ") {
                let found_type = json_type(unexpected);
                ParseRequestBodyProblem::type_mismatch(
                    path,
                    message.as_str(),
                    None::<String>,
                    found_type,
                    None,
                    line,
                    column,
                )
            } else {
                ParseRequestBodyProblem::deserialize(path, message, None::<String>, line, column)
            }
        }
        Category::Io => {
            ParseRequestBodyProblem::deserialize(path, message, None::<String>, line, column)
        }
    }
}

/// The JSON type at the start of a serde `invalid type: ...` message, for example `string "abc"`.
fn json_type(unexpected: &str) -> Option<JsonType> {
    [
        ("string", JsonType::String),
        ("boolean", JsonType::Bool),
        ("floating point", JsonType::Float),
        ("integer", JsonType::Integer),
        ("sequence", JsonType::Array),
        ("map", JsonType::Object),
        ("null", JsonType::Null),
    ]
    .into_iter()
    .find(|(prefix, _)| unexpected.starts_with(prefix))
    .map(|(_, json_type)| json_type)
}

/// The faults of the server that each operation can return. The in-memory store of the example never fails, but
/// the store of a real service can.
#[problem_set(documents(InternalServerErrorProblem, DependencyProblem))]
#[derive(Debug)]
enum ServerError {}

// The operations. Each operation has a problem set. A variant holds a problem that the handler returns.
// `documents(...)` lists the problems of the extractors, which reject the request before the handler runs.

/// The problems of `GET /widgets`.
#[problem_set(documents(UnauthorizedProblem, QueryError, ServerError))]
#[derive(Debug)]
enum ListWidgetsError {
    Forbidden(ForbiddenProblem),
}

/// Lists the widgets.
#[utoipa::path(
    get,
    path = "/widgets",
    params(("limit" = Option<u32>, Query, description = "The maximum number of widgets. The default is 20.")),
    responses((status = 200, description = "The widgets, by identifier", body = Vec<Widget>), ListWidgetsError),
)]
async fn list_widgets(
    State(state): State<AppState>,
    caller: Caller,
    page: Page,
) -> Result<Json<Vec<Widget>>, ListWidgetsError> {
    caller.require("widgets:read")?;
    let widgets = state.widgets.lock().unwrap();
    Ok(Json(
        widgets
            .values()
            .take(page.limit as usize)
            .cloned()
            .collect(),
    ))
}

/// The problems of `GET /widgets/{id}`.
#[problem_set(documents(UnauthorizedProblem, PathError, ServerError))]
#[derive(Debug)]
enum GetWidgetError {
    Forbidden(ForbiddenProblem),
    UnknownResource(UnknownResourceProblem),
}

/// Returns one widget.
#[utoipa::path(
    get,
    path = "/widgets/{id}",
    params(("id" = u64, Path, description = "The identifier of the widget")),
    responses((status = 200, description = "The widget", body = Widget), GetWidgetError),
)]
async fn get_widget(
    State(state): State<AppState>,
    caller: Caller,
    WidgetId(id): WidgetId,
) -> Result<Json<Widget>, GetWidgetError> {
    caller.require("widgets:read")?;
    let widgets = state.widgets.lock().unwrap();
    let widget = widgets
        .get(&id)
        .ok_or_else(|| UnknownResourceProblem::new("widget", id))?;
    Ok(Json(widget.clone()))
}

/// The problems of `POST /widgets`.
#[problem_set(documents(UnauthorizedProblem, BodyError, ServerError))]
#[derive(Debug)]
enum CreateWidgetError {
    Forbidden(ForbiddenProblem),
    NameTaken(WidgetNameTaken),
}

/// Creates a widget.
#[utoipa::path(
    post,
    path = "/widgets",
    request_body = NewWidget,
    responses((status = 201, description = "The new widget", body = Widget), CreateWidgetError),
)]
async fn create_widget(
    State(state): State<AppState>,
    caller: Caller,
    JsonBody(new): JsonBody<NewWidget>,
) -> Result<(StatusCode, Json<Widget>), CreateWidgetError> {
    caller.require("widgets:write")?;
    let mut widgets = state.widgets.lock().unwrap();
    if let Some(existing) = widgets.values().find(|widget| widget.name == new.name) {
        return Err(WidgetNameTaken {
            name: new.name,
            existing_id: existing.id,
        }
        .into());
    }
    let id = widgets.keys().next_back().map_or(1, |last| last + 1);
    let widget = Widget { id, name: new.name };
    widgets.insert(id, widget.clone());
    Ok((StatusCode::CREATED, Json(widget)))
}

// The application.

/// The Openapi document of the server.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Widgets",
        version = "1.0.0",
        description = "An example service of `problem-details-fine`."
    ),
    paths(list_widgets, get_widget, create_widget)
)]
pub struct ApiDoc;

/// The router of the server.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/widgets", get(list_widgets).post(create_widget))
        .route("/widgets/{id}", get(get_widget))
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .fallback(|| async { UnknownPathOperationProblem })
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;

    /// Sends a request and returns the status, the `content-type` and the JSON body.
    async fn send(app: &Router, request: Request<Body>) -> (StatusCode, String, Value) {
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let content_type = response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .to_owned();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, content_type, serde_json::from_slice(&body).unwrap())
    }

    fn get(uri: &str, token: Option<&str>) -> Request<Body> {
        let request = Request::get(uri);
        let request = match token {
            Some(token) => request.header(header::AUTHORIZATION, format!("Bearer {token}")),
            None => request,
        };
        request.body(Body::empty()).unwrap()
    }

    fn post(body: &'static str, content_type: Option<&str>, token: &str) -> Request<Body> {
        let request =
            Request::post("/widgets").header(header::AUTHORIZATION, format!("Bearer {token}"));
        let request = match content_type {
            Some(content_type) => request.header(header::CONTENT_TYPE, content_type),
            None => request,
        };
        request.body(Body::from(body)).unwrap()
    }

    const JSON: Option<&str> = Some("application/json");

    #[tokio::test]
    async fn create_and_get() {
        let app = app(AppState::default());

        let (status, _, body) =
            send(&app, post(r#"{"name": "sprocket"}"#, JSON, "writer-token")).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body, json!({"id": 1, "name": "sprocket"}));

        let (status, _, body) = send(&app, get("/widgets/1", Some("reader-token"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, json!({"id": 1, "name": "sprocket"}));

        let (status, content_type, body) =
            send(&app, post(r#"{"name": "sprocket"}"#, JSON, "writer-token")).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(content_type, "application/problem+json");
        assert_eq!(
            body["type"],
            "https://widgets.example.com/problems/name-taken"
        );
        assert_eq!(body["existing_id"], 1);
    }

    #[tokio::test]
    async fn problems() {
        let app = app(AppState::default());
        let name = r#"{"name": "x"}"#;
        let cases = [
            (get("/widgets", None), "UNAUTHORIZED"),
            (get("/widgets", Some("wrong-token")), "UNAUTHORIZED"),
            (post(name, JSON, "reader-token"), "FORBIDDEN"),
            (
                get("/widgets/abc", Some("reader-token")),
                "PARSE_PATH_PARAMETER",
            ),
            (
                get("/widgets/%FF", Some("reader-token")),
                "PARSE_PATH_PARAMETER",
            ),
            (get("/widgets/7", Some("reader-token")), "UNKNOWN_RESOURCE"),
            (
                get("/widgets?limit=many", Some("reader-token")),
                "PARSE_QUERY_PARAMETER",
            ),
            (post(name, None, "writer-token"), "MISSING_MIME"),
            (
                post(name, Some("text/plain"), "writer-token"),
                "UNSUPPORTED_MIME",
            ),
            (post(name, Some("json"), "writer-token"), "PARSE_MIME"),
            (post("", JSON, "writer-token"), "MISSING_REQUEST_BODY"),
            (
                post(r#"{"name": 7}"#, JSON, "writer-token"),
                "PARSE_REQUEST_BODY",
            ),
            (get("/gadgets", None), "UNKNOWN_PATH_OPERATION"),
        ];
        for (request, problem) in cases {
            let uri = request.uri().clone();
            let (_, content_type, body) = send(&app, request).await;
            assert_eq!(
                body["type"],
                format!("tag:veeg.dev,2026:{problem}"),
                "{uri}: {body}"
            );
            assert_eq!(content_type, "application/problem+json");
        }
    }

    #[tokio::test]
    async fn request_body_problems() {
        let app = app(AppState::default());

        let (_, _, body) = send(&app, post("{}", JSON, "writer-token")).await;
        assert_eq!(body["kind"], "MISSING_REQUIRED_FIELD");
        assert_eq!(body["field"], "name");

        let (_, _, body) = send(&app, post(r#"{"name": 7}"#, JSON, "writer-token")).await;
        assert_eq!(body["kind"], "TYPE_MISMATCH");
        assert_eq!(body["field"], "name");
        assert_eq!(body["found_type"], "INTEGER");

        let (_, _, body) = send(&app, post(r#"{"name": "#, JSON, "writer-token")).await;
        assert_eq!(body["kind"], "SYNTAX");
    }
}
