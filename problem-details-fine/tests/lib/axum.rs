//! `IntoResponse` for problem types, problem sets and documents.

use crate::types::{FlatProblem, UnitProblem};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use problem_details_fine::{ProblemDetails, problem_set};
use serde_json::{Value, json};
use tower::ServiceExt;

#[problem_set]
#[derive(Debug)]
enum HandlerErrors {
    Conflict(FlatProblem),
    Missing(UnitProblem),
}

async fn handler() -> Result<&'static str, HandlerErrors> {
    Err(FlatProblem {
        widget_id: "w".into(),
        count: 1,
        note: None,
    })?
}

async fn parts(response: Response) -> (StatusCode, Option<String>, Value) {
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().unwrap().to_owned());
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, content_type, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn handler_returning_a_problem_set() {
    let app = Router::new().route("/", get(handler));
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let (status, content_type, body) = parts(response).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(content_type.as_deref(), Some("application/problem+json"));
    assert_eq!(body["type"], "https://fixtures.test/problems/flat");
    assert_eq!(body["widgetId"], "w");
}

#[tokio::test]
async fn problem_type_and_documents() {
    let (status, content_type, body) = parts(UnitProblem.into_response()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(content_type.as_deref(), Some("application/problem+json"));
    assert_eq!(
        body,
        json!({"type": "tag:fixtures.test,2025:UNIT", "status": 404, "title": "Unit"})
    );

    let (_, _, body) = parts(HandlerErrors::from(UnitProblem).into_response()).await;
    assert_eq!(body["status"], 404);

    let (_, _, body) = parts(
        ProblemDetails::from(UnitProblem)
            .with_instance("/x")
            .into_response(),
    )
    .await;
    assert_eq!(body["instance"], "/x");

    let (status, _, body) =
        parts(ProblemDetails::new(StatusCode::BAD_GATEWAY).into_response()).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(
        body,
        json!({"type": "about:blank", "status": 502, "title": "Bad Gateway"})
    );
}

/// A problem type whose extension members fail to serialize: a bug in the type.
#[problem_details_fine::problem_type(
    type = "tag:fixtures.test,2025:BROKEN",
    status = CONFLICT,
    title = "Broken"
)]
#[derive(Debug)]
struct BrokenProblem {
    #[serde(serialize_with = "fail")]
    value: u32,
}

fn fail<S: serde::Serializer>(_: &u32, _: S) -> Result<S::Ok, S::Error> {
    Err(serde::ser::Error::custom("boom"))
}

#[tokio::test]
async fn unserializable_problem_falls_back_to_internal_server_error() {
    let expected = json!({"type": "about:blank", "status": 500, "title": "Internal Server Error"});
    for response in [
        BrokenProblem { value: 1 }.into_response(),
        ProblemDetails::from(BrokenProblem { value: 1 }).into_response(),
    ] {
        let (status, content_type, body) = parts(response).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(content_type.as_deref(), Some("application/problem+json"));
        assert_eq!(body, expected);
    }
}
