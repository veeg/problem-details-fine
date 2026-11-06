//! `omit(..)` of `#[problem_type]`: the schema leaves out `detail` and `instance`.

use problem_details_fine::{problem_set, problem_type};
use serde_json::{Value, json};
use utoipa::{IntoResponses, PartialSchema};

/// A problem type that never has `detail` or `instance`.
#[problem_type(type = "tag:fixtures.test,2025:OMIT_BOTH", status = NOT_FOUND, title = "Both", omit(detail, instance))]
#[derive(Debug, Clone)]
struct OmitBoth;

/// A problem type with a `detail`, but never an `instance`.
#[problem_type(
    type = "tag:fixtures.test,2025:OMIT_INSTANCE",
    status = CONFLICT,
    title = "Instance",
    detail = "widget `{id}` exists",
    omit(instance),
)]
#[derive(Debug, Clone)]
struct OmitInstance {
    id: String,
}

/// A problem type with extension members under `error`, that never has `detail`.
#[problem_type(
    type = "tag:fixtures.test,2025:OMIT_WRAPPED",
    status = BAD_REQUEST,
    title = "Wrapped",
    wrap = "error",
    omit(detail),
)]
#[derive(Debug, Clone)]
struct OmitWrapped {
    code: u32,
}

/// A problem type without `omit`.
#[problem_type(type = "tag:fixtures.test,2025:OMIT_NONE", status = NOT_FOUND, title = "None")]
#[derive(Debug, Clone)]
struct OmitNone;

/// Returns the names of the properties of an object schema.
fn properties(schema: &Value) -> Vec<&str> {
    schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

fn schema<T: PartialSchema>() -> Value {
    serde_json::to_value(T::schema()).unwrap()
}

#[test]
fn omit_both_leaves_out_detail_and_instance() {
    assert_eq!(
        properties(&schema::<OmitBoth>()),
        ["status", "title", "type"]
    );
}

#[test]
fn omit_instance_keeps_detail() {
    assert_eq!(
        properties(&schema::<OmitInstance>()),
        ["detail", "id", "status", "title", "type"]
    );
}

#[test]
fn omit_detail_with_wrapped_members() {
    assert_eq!(
        properties(&schema::<OmitWrapped>()),
        ["error", "instance", "status", "title", "type"]
    );
}

#[test]
fn without_omit_the_schema_has_both() {
    assert_eq!(
        properties(&schema::<OmitNone>()),
        ["detail", "instance", "status", "title", "type"]
    );
}

#[test]
fn problem_set_uses_the_schema_without_the_members() {
    #[problem_set]
    #[derive(Debug)]
    enum Errors {
        Both(OmitBoth),
        None(OmitNone),
    }

    let responses = serde_json::to_value(Errors::responses()).unwrap();
    let one_of = &responses["404"]["content"]["application/problem+json"]["schema"]["oneOf"];
    let by_title = |title: &str| {
        one_of
            .as_array()
            .unwrap()
            .iter()
            .find(|schema| schema["title"] == json!(title))
            .unwrap()
            .clone()
    };
    assert_eq!(
        properties(&by_title("OMIT_BOTH")),
        ["status", "title", "type"]
    );
    assert_eq!(
        properties(&by_title("OMIT_NONE")),
        ["detail", "instance", "status", "title", "type"]
    );
}

#[test]
fn omit_does_not_change_the_document() {
    assert_eq!(
        serde_json::to_value(OmitBoth).unwrap(),
        json!({"type": "tag:fixtures.test,2025:OMIT_BOTH", "status": 404, "title": "Both"})
    );
    assert_eq!(
        serde_json::to_value(OmitInstance { id: "w".into() }).unwrap(),
        json!({
            "type": "tag:fixtures.test,2025:OMIT_INSTANCE",
            "status": 409,
            "title": "Instance",
            "detail": "widget `w` exists",
            "id": "w",
        })
    );
}
