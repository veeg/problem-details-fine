use axum::{
    body::Body,
    http::{HeaderValue, Response, StatusCode, header},
    response::IntoResponse,
};
use serde::Serialize;

use crate::{MEDIA_TYPE, ProblemDetails};

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response<Body> {
        into_response(self.status(), &self)
    }
}

/// The document that the response contains if a problem does not serialize. This document always serializes,
/// and it does not show the cause.
const FALLBACK: &str = r#"{"type":"about:blank","status":500,"title":"Internal Server Error"}"#;

/// Makes an `application/problem+json` response with `status`. It serializes `document` one time.
///
/// Serialization fails only if the extension members of a problem type do not serialize. That is a bug in the
/// problem type. The handler is already complete, thus the response becomes `500 Internal Server Error` with an
/// `about:blank` document.
///
/// The `IntoResponse` implementations from the macros call this function.
pub fn into_response<T: Serialize>(status: StatusCode, document: &T) -> Response<Body> {
    let (status, body) = match serde_json::to_vec(document) {
        Ok(body) => (status, Body::from(body)),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Body::from(FALLBACK)),
    };
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(MEDIA_TYPE));
    response
}
