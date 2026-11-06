//! Snapshots of the schema and a serialized document of every embedded problem type.

use problem_details_fine::{
    ProblemDetails, ProblemType,
    embedded::{
        DependencyProblem, ForbiddenProblem, InternalServerErrorProblem, JsonType,
        MissingMimeProblem, MissingRequestBodyProblem, ParseMimeProblem, ParsePathParameterProblem,
        ParseQueryParameterProblem, ParseRequestBodyProblem, UnauthorizedProblem,
        UnhandledPathParameterErrorProblem, UnhandledQueryStringErrorProblem,
        UnhandledRequestBodyErrorProblem, UnknownPathOperationProblem, UnknownResourceProblem,
        UnsupportedMimeProblem,
    },
};
use utoipa::PartialSchema;

#[track_caller]
fn check<T: ProblemType + PartialSchema>(name: &str, problem: T) {
    let schema = serde_json::to_value(T::schema()).unwrap();
    let document = serde_json::to_value(&problem).unwrap();
    assert_eq!(
        serde_json::to_value(ProblemDetails::from(problem)).unwrap(),
        document
    );
    assert!(
        jsonschema::is_valid(&schema, &document),
        "{name} does not conform: {document:#}"
    );
    insta::assert_json_snapshot!(format!("{name}-schema"), schema);
    insta::assert_json_snapshot!(format!("{name}-document"), document);
}

#[test]
fn internal_server_error() {
    check("internal_server_error", InternalServerErrorProblem);
}

#[test]
fn unauthorized() {
    check("unauthorized", UnauthorizedProblem::MissingAuthorization);
    check(
        "unauthorized_scheme",
        UnauthorizedProblem::deprecated_scheme("Basic", &["Bearer", "DPoP"]),
    );
}

#[test]
fn forbidden() {
    check("forbidden", ForbiddenProblem::new(&["widgets:write"]));
    assert_eq!(ForbiddenProblem::new(&[] as &[&str]).detail(), None);
}

#[test]
fn unsupported_mime() {
    check(
        "unsupported_mime",
        UnsupportedMimeProblem::new("content-type", "text/plain", &["application/json"]),
    );
}

#[test]
fn missing_mime() {
    check("missing_mime", MissingMimeProblem::new("content-type"));
}

#[test]
fn parse_mime() {
    check(
        "parse_mime",
        ParseMimeProblem::invalid_ascii("content-type", vec![0xff]),
    );
    check(
        "parse_mime_error",
        ParseMimeProblem::parse_error("accept", "bad", b"x/".to_vec()),
    );
}

#[test]
fn missing_request_body() {
    check("missing_request_body", MissingRequestBodyProblem);
}

#[test]
fn unknown_path_operation() {
    check("unknown_path_operation", UnknownPathOperationProblem);
}

#[test]
fn parse_path_parameter() {
    check(
        "parse_path_parameter",
        ParsePathParameterProblem::parse_error("abc", "u32"),
    );
    check(
        "parse_path_parameter_deserialize",
        ParsePathParameterProblem::deserialize("id", "abc", "invalid digit"),
    );
}

#[test]
fn parse_query_parameter() {
    check(
        "parse_query_parameter",
        ParseQueryParameterProblem::missing_required_field("limit"),
    );
    check(
        "parse_query_parameter_deserialize",
        ParseQueryParameterProblem::deserialize("limit", "invalid digit", Some("x"), None::<&str>),
    );
}

#[test]
fn parse_request_body() {
    check(
        "parse_request_body",
        ParseRequestBodyProblem::missing_required_field("name"),
    );
    check(
        "parse_request_body_type_mismatch",
        ParseRequestBodyProblem::type_mismatch(
            "name",
            "expected a string",
            Some("1"),
            Some(JsonType::Integer),
            Some(JsonType::String),
            1,
            9,
        ),
    );
    check(
        "parse_request_body_deserialize",
        ParseRequestBodyProblem::deserialize("name", "bad", None::<&str>, 1, 2),
    );
    check(
        "parse_request_body_syntax",
        ParseRequestBodyProblem::syntax(".", "EOF", 1, 1),
    );
}

#[test]
fn unhandled_errors() {
    check(
        "unhandled_request_body_error",
        UnhandledRequestBodyErrorProblem,
    );
    check(
        "unhandled_path_parameter_error",
        UnhandledPathParameterErrorProblem,
    );
    check(
        "unhandled_query_string_error",
        UnhandledQueryStringErrorProblem,
    );
}

#[test]
fn dependency() {
    check("dependency", DependencyProblem);
}

#[test]
fn unknown_resource() {
    check(
        "unknown_resource",
        UnknownResourceProblem::new("widget", "w-1"),
    );
}
