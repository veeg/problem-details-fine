//! An `axum` server for widgets. Each handler returns a problem set, and the Openapi document shows each problem
//! type that an operation can return. See `failed-requests.md` in this directory for the response of each failed request.
//!
//! Start the server on `127.0.0.1:3000`. The server prints each request and the status of its response. For a
//! problem, it also prints the `type`, the `kind` and the `detail`:
//!
//! ```text
//! cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types
//! ```
//!
//! While the server runs, send a list of failed requests to it and print each response. The default address is
//! `127.0.0.1:3000`:
//!
//! ```text
//! cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types -- showcase [ADDRESS]
//! ```
//!
//! Write the Openapi document to `examples/widgets/openapi.json`, the responses of a problem set with each problem
//! type of the `embedded` module as HTML to `examples/widgets/responses.html`, and the output of `showcase` to
//! `examples/widgets/failed-requests.md`. The command starts the server on a free port and runs `showcase` against
//! it. The documentation of the crate includes `responses.html`:
//!
//! ```text
//! cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types -- generate
//! ```
//!
//! The server also serves the Openapi document on `GET /openapi.json`. The tests `openapi_document_is_current`,
//! `responses_html_is_current` and `failed_requests_are_current` fail if a file is not the same as the generated
//! output.
//!
//! The server accepts two bearer tokens. `reader-token` has the scope `widgets:read`. `writer-token` also has the
//! scope `widgets:write`.
//!
//! ```text
//! curl -i -H 'Authorization: Bearer writer-token' -H 'Content-Type: application/json' \
//!     -d '{"name": "sprocket"}' http://127.0.0.1:3000/widgets
//! curl -i -H 'Authorization: Bearer reader-token' http://127.0.0.1:3000/widgets/abc
//! ```
mod api;
mod responses;

use axum::http::StatusCode;
use problem_details_fine::embedded;
use utoipa::OpenApi;

use crate::api::{ApiDoc, AppState, app};

/// The path of the Openapi document in the repository.
const OPENAPI_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/widgets/openapi.json");

/// The path of the problem responses of the Openapi document, as HTML.
const RESPONSES_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/widgets/responses.html"
);

/// The path of the generated file with the output of `showcase`.
const FAILED_REQUESTS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/widgets/failed-requests.md"
);

/// The Openapi document, as pretty-printed JSON with a final newline.
fn openapi_json() -> String {
    ApiDoc::openapi()
        .to_pretty_json()
        .expect("the Openapi document serializes")
        + "\n"
}

/// Each problem type of the `embedded` module. The documentation of the crate shows the responses of this set.
#[problem_details_fine::problem_set(documents(
    embedded::InternalServerErrorProblem,
    embedded::UnauthorizedProblem,
    embedded::ForbiddenProblem,
    embedded::UnsupportedMimeProblem,
    embedded::MissingMimeProblem,
    embedded::ParseMimeProblem,
    embedded::MissingRequestBodyProblem,
    embedded::UnknownPathOperationProblem,
    embedded::ParsePathParameterProblem,
    embedded::ParseQueryParameterProblem,
    embedded::ParseRequestBodyProblem,
    embedded::UnhandledRequestBodyErrorProblem,
    embedded::UnhandledPathParameterErrorProblem,
    embedded::UnhandledQueryStringErrorProblem,
    embedded::DependencyProblem,
    embedded::UnknownResourceProblem,
))]
#[derive(Debug)]
enum EmbeddedProblems {}

/// The responses of [`EmbeddedProblems`], as HTML.
fn responses_html() -> String {
    let responses = serde_json::to_value(<EmbeddedProblems as utoipa::IntoResponses>::responses())
        .expect("the responses serialize");
    responses::render(responses.as_object().expect("the responses are a map"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => serve().await,
        Some("showcase") => {
            let address = args.get(1).map_or(DEFAULT_ADDRESS, String::as_str);
            match showcase(address).await {
                Ok(output) => print!("{output}"),
                Err(error) => {
                    eprintln!(
                        "the showcase failed against `{address}`: {error}\nStart the server with `cargo run -p \
                         problem-details-fine --example widgets --features axum,utoipa,embedded-types`."
                    );
                    std::process::exit(1);
                }
            }
        }
        Some("generate") => {
            std::fs::write(OPENAPI_PATH, openapi_json()).expect("write the Openapi document");
            println!("wrote {OPENAPI_PATH}");
            std::fs::write(RESPONSES_PATH, responses_html()).expect("write the responses");
            println!("wrote {RESPONSES_PATH}");
            std::fs::write(
                FAILED_REQUESTS_PATH,
                failed_requests_md(&showcase_on_new_server().await),
            )
            .expect("write the failed requests");
            println!("wrote {FAILED_REQUESTS_PATH}");
        }
        Some(other) => {
            eprintln!("unknown command `{other}`; expected no command, `showcase` or `generate`");
            std::process::exit(2);
        }
    }
}

/// The address of the server, and the default address of `showcase`.
const DEFAULT_ADDRESS: &str = "127.0.0.1:3000";

async fn serve() {
    let listener = tokio::net::TcpListener::bind(DEFAULT_ADDRESS)
        .await
        .unwrap_or_else(|error| panic!("bind {DEFAULT_ADDRESS}: {error}"));
    println!("listening on http://{DEFAULT_ADDRESS}");
    let app = app(AppState::default()).layer(axum::middleware::from_fn(log_request));
    axum::serve(listener, app).await.expect("serve");
}

/// Prints the method and the URI of each request, and the status of its response. For a problem details document,
/// it also prints the `type`, the `kind` and the `detail` of the problem.
async fn log_request(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let (method, uri) = (request.method().clone(), request.uri().clone());
    let response = next.run(request).await;
    let status = response.status();
    println!(
        "{method} {uri} -> {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or_default()
    );
    if response.headers().get(axum::http::header::CONTENT_TYPE)
        != Some(&axum::http::HeaderValue::from_static(
            problem_details_fine::MEDIA_TYPE,
        ))
    {
        return response;
    }
    let (parts, body) = response.into_parts();
    let body = axum::body::to_bytes(body, usize::MAX)
        .await
        .expect("read the response body");
    let document: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
    for member in ["type", "kind", "detail"] {
        if let Some(value) = document[member].as_str() {
            println!("    {member}: {value}");
        }
    }
    axum::response::Response::from_parts(parts, axum::body::Body::from(body))
}

/// Starts the server on a free port of `127.0.0.1`, and runs `showcase` against it.
async fn showcase_on_new_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind a free port");
    let address = listener
        .local_addr()
        .expect("the address of the listener")
        .to_string();
    let server = tokio::spawn(async move { axum::serve(listener, app(AppState::default())).await });
    let output = showcase(&address)
        .await
        .expect("the showcase runs against the new server");
    server.abort();
    output
}

// The showcase.

/// A failed request in the showcase.
struct Case {
    title: &'static str,
    method: &'static str,
    uri: &'static str,
    headers: &'static [(&'static str, &'static [u8])],
    body: &'static str,
}

const READER: (&str, &[u8]) = ("authorization", b"Bearer reader-token");
const WRITER: (&str, &[u8]) = ("authorization", b"Bearer writer-token");
const JSON: (&str, &[u8]) = ("content-type", b"application/json");
/// A body for `POST /widgets`. Before the cases, the showcase makes sure that a widget with the name `sprocket`
/// exists.
const NEW_WIDGET: &str = r#"{"name": "gear"}"#;

const CASES: &[Case] = &[
    Case {
        title: "No `Authorization` header",
        method: "GET",
        uri: "/widgets",
        headers: &[],
        body: "",
    },
    Case {
        title: "Malformed `Authorization` header",
        method: "GET",
        uri: "/widgets",
        headers: &[("authorization", b"Bearer caf\xe9")],
        body: "",
    },
    Case {
        title: "Unsupported authentication scheme",
        method: "GET",
        uri: "/widgets",
        headers: &[("authorization", b"Basic dXNlcjpwYXNz")],
        body: "",
    },
    Case {
        title: "No credentials",
        method: "GET",
        uri: "/widgets",
        headers: &[("authorization", b"Bearer")],
        body: "",
    },
    Case {
        title: "Unknown token",
        method: "GET",
        uri: "/widgets",
        headers: &[("authorization", b"Bearer wrong-token")],
        body: "",
    },
    Case {
        title: "Token without the required scope",
        method: "POST",
        uri: "/widgets",
        headers: &[READER, JSON],
        body: NEW_WIDGET,
    },
    Case {
        title: "Query parameter of the wrong type",
        method: "GET",
        uri: "/widgets?limit=many",
        headers: &[READER],
        body: "",
    },
    Case {
        title: "Path parameter of the wrong type",
        method: "GET",
        uri: "/widgets/abc",
        headers: &[READER],
        body: "",
    },
    Case {
        title: "Path parameter that is not UTF-8",
        method: "GET",
        uri: "/widgets/%FF",
        headers: &[READER],
        body: "",
    },
    Case {
        title: "Unknown widget",
        method: "GET",
        uri: "/widgets/999999",
        headers: &[READER],
        body: "",
    },
    Case {
        title: "No `Content-Type` header",
        method: "POST",
        uri: "/widgets",
        headers: &[WRITER],
        body: NEW_WIDGET,
    },
    Case {
        title: "Unsupported media type",
        method: "POST",
        uri: "/widgets",
        headers: &[WRITER, ("content-type", b"text/plain")],
        body: NEW_WIDGET,
    },
    Case {
        title: "`Content-Type` that is not a media type",
        method: "POST",
        uri: "/widgets",
        headers: &[WRITER, ("content-type", b"json")],
        body: NEW_WIDGET,
    },
    Case {
        title: "`Content-Type` with characters that are not visible ASCII",
        method: "POST",
        uri: "/widgets",
        headers: &[WRITER, ("content-type", b"application/js\xf6n")],
        body: NEW_WIDGET,
    },
    Case {
        title: "No request body",
        method: "POST",
        uri: "/widgets",
        headers: &[WRITER, JSON],
        body: "",
    },
    Case {
        title: "JSON syntax error",
        method: "POST",
        uri: "/widgets",
        headers: &[WRITER, JSON],
        body: r#"{"name": "gear""#,
    },
    Case {
        title: "Missing field",
        method: "POST",
        uri: "/widgets",
        headers: &[WRITER, JSON],
        body: "{}",
    },
    Case {
        title: "Field of the wrong type",
        method: "POST",
        uri: "/widgets",
        headers: &[WRITER, JSON],
        body: r#"{"name": 7}"#,
    },
    Case {
        title: "Name that another widget has",
        method: "POST",
        uri: "/widgets",
        headers: &[WRITER, JSON],
        body: r#"{"name": "sprocket"}"#,
    },
    Case {
        title: "Unknown path",
        method: "GET",
        uri: "/gadgets",
        headers: &[READER],
        body: "",
    },
];

/// The response to a case.
struct Outcome {
    status: StatusCode,
    content_type: String,
    body: String,
}

/// Sends each case to the server at `address` and returns a Markdown summary and each request with its response.
async fn showcase(address: &str) -> std::io::Result<String> {
    let setup = exchange(
        address,
        "POST",
        "/widgets",
        &[WRITER, JSON],
        r#"{"name": "sprocket"}"#,
    )
    .await?;
    if !matches!(setup.status, StatusCode::CREATED | StatusCode::CONFLICT) {
        return Err(invalid_data(format!(
            "`POST /widgets` for `sprocket` returned {}",
            setup.status
        )));
    }

    let mut outcomes = Vec::new();
    for case in CASES {
        outcomes.push(exchange(address, case.method, case.uri, case.headers, case.body).await?);
    }

    let mut out = String::new();
    out.push_str("| Request | Status | `type` | `kind` |\n|---|---|---|---|\n");
    for (case, outcome) in CASES.iter().zip(&outcomes) {
        let document: serde_json::Value = serde_json::from_str(&outcome.body).unwrap();
        let kind = document["kind"]
            .as_str()
            .map_or(String::new(), |kind| format!("`{kind}`"));
        out.push_str(&format!(
            "| {} | {} | `{}` | {kind} |\n",
            case.title,
            outcome.status.as_u16(),
            document["type"].as_str().unwrap(),
        ));
    }
    for (case, outcome) in CASES.iter().zip(&outcomes) {
        out.push_str(&format!(
            "\n## {}\n\n```http\n{} {} HTTP/1.1\n",
            case.title, case.method, case.uri
        ));
        for (name, value) in case.headers {
            out.push_str(&format!("{name}: {}\n", escape_bytes(value)));
        }
        if !case.body.is_empty() {
            out.push_str(&format!("\n{}\n", case.body));
        }
        out.push_str(&format!(
            "```\n\n```http\nHTTP/1.1 {} {}\ncontent-type: {}\n\n{}\n```\n",
            outcome.status.as_u16(),
            outcome.status.canonical_reason().unwrap_or_default(),
            outcome.content_type,
            indent_json(&outcome.body),
        ));
    }
    Ok(out)
}

/// Sends one HTTP/1.1 request on a new connection, and reads the response until the server closes the connection.
async fn exchange(
    address: &str,
    method: &str,
    uri: &str,
    headers: &[(&str, &[u8])],
    body: &str,
) -> std::io::Result<Outcome> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut request = format!(
        "{method} {uri} HTTP/1.1\r\nhost: {address}\r\nconnection: close\r\ncontent-length: {}\r\n",
        body.len()
    )
    .into_bytes();
    for (name, value) in headers {
        request.extend_from_slice(format!("{name}: ").as_bytes());
        request.extend_from_slice(value);
        request.extend_from_slice(b"\r\n");
    }
    request.extend_from_slice(b"\r\n");
    request.extend_from_slice(body.as_bytes());

    let mut stream = tokio::net::TcpStream::connect(address).await?;
    stream.write_all(&request).await?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;
    parse_response(&response)
}

/// Reads the status, the `content-type` and the body of a response with a `content-length`.
fn parse_response(response: &[u8]) -> std::io::Result<Outcome> {
    let response =
        std::str::from_utf8(response).map_err(|error| invalid_data(error.to_string()))?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| invalid_data("the response has no end of the header section".into()))?;
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .and_then(|code| StatusCode::from_u16(code).ok())
        .ok_or_else(|| invalid_data("the response has no status line".into()))?;
    let header = |name: &str| {
        head.split("\r\n").skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name)
                .then(|| value.trim().to_owned())
        })
    };
    if header("transfer-encoding").is_some() {
        return Err(invalid_data(
            "the showcase reads only responses with a `content-length`".into(),
        ));
    }
    let content_type = header("content-type").unwrap_or_default();
    Ok(Outcome {
        status,
        content_type,
        body: body.to_owned(),
    })
}

fn invalid_data(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

/// Writes visible ASCII and spaces as they are, and other bytes as `\xNN`.
fn escape_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| match byte {
            b' '..=b'~' => char::from(byte).to_string(),
            _ => format!("\\x{byte:02X}"),
        })
        .collect()
}

/// Indents compact JSON with two spaces. Unlike `serde_json::to_string_pretty` of a `Value`, it keeps the order of
/// the members as the server wrote them.
fn indent_json(json: &str) -> String {
    let newline = |out: &mut String, depth: usize| {
        out.push('\n');
        out.push_str(&"  ".repeat(depth));
    };
    let mut out = String::new();
    let mut depth = 0;
    let (mut in_string, mut escaped) = (false, false);
    let mut chars = json.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '{' | '[' if matches!(chars.peek(), Some('}' | ']')) => {
                out.push(c);
                out.push(chars.next().unwrap());
            }
            '{' | '[' => {
                out.push(c);
                depth += 1;
                newline(&mut out, depth);
            }
            '}' | ']' => {
                depth -= 1;
                newline(&mut out, depth);
                out.push(c);
            }
            ',' => {
                out.push(c);
                newline(&mut out, depth);
            }
            ':' => out.push_str(": "),
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    out
}

/// The content of `failed-requests.md`: a heading and a notice, followed by the output of `showcase`.
fn failed_requests_md(showcase: &str) -> String {
    format!(
        "# Failed requests\n\n<!-- Generated by `cargo run -p problem-details-fine --example widgets --features \
         axum,utoipa,embedded-types -- generate`. Do not edit. -->\n\n{showcase}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_document_is_current() {
        let committed = std::fs::read_to_string(OPENAPI_PATH).unwrap_or_default();
        assert!(
            committed == openapi_json(),
            "`examples/widgets/openapi.json` is out of date. Run \
             `cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types -- generate`."
        );
    }

    #[test]
    fn responses_html_is_current() {
        let committed = std::fs::read_to_string(RESPONSES_PATH).unwrap_or_default();
        assert!(
            committed == responses_html(),
            "`examples/widgets/responses.html` is out of date. Run \
             `cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types -- generate`."
        );
    }

    #[tokio::test]
    async fn failed_requests_are_current() {
        let committed = std::fs::read_to_string(FAILED_REQUESTS_PATH).unwrap_or_default();
        assert!(
            committed == failed_requests_md(&showcase_on_new_server().await),
            "`examples/widgets/failed-requests.md` is out of date. Run \
             `cargo run -p problem-details-fine --example widgets --features axum,utoipa,embedded-types -- generate`."
        );
    }

    #[test]
    fn indent_json_keeps_member_order() {
        assert_eq!(
            indent_json(r#"{"b":1,"a":[],"c":{"d":"x,\"{"}}"#),
            "{\n  \"b\": 1,\n  \"a\": [],\n  \"c\": {\n    \"d\": \"x,\\\"{\"\n  }\n}"
        );
    }
}
