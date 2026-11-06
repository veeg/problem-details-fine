//! Openapi documents generated from problem types and problem sets.

use crate::types::{
    ComputedProblem, ExplicitTagProblem, ExternalProblem, FlatProblem, NewtypeProblem,
    TaggedProblem, UnitProblem, WrappedProblem,
};
use problem_details_fine::problem_set;
use serde_json::{Value, json};
use utoipa::{IntoResponses, OpenApi};

#[problem_set]
#[derive(Debug)]
pub enum AuthErrors {
    Missing(UnitProblem),
    Throttled(ComputedProblem),
}

/// Problems produced before a handler runs; `UnitProblem` overlaps with `AuthErrors`.
#[problem_set]
#[derive(Debug)]
pub enum CommonErrors {
    Missing(UnitProblem),
    Wrapped(WrappedProblem),
}

#[problem_set(documents(ExplicitTagProblem, CommonErrors))]
#[derive(Debug)]
pub enum EndpointErrors {
    Conflict(FlatProblem),
    Tagged(TaggedProblem),
    Auth(AuthErrors),
}

#[utoipa::path(post, path = "/widgets", responses((status = 201, description = "Created"), EndpointErrors))]
#[allow(dead_code)]
fn create_widget() {}

#[utoipa::path(get, path = "/widgets/{id}", responses((status = 200, description = "Ok"), UnitProblem))]
#[allow(dead_code)]
fn get_widget() {}

#[utoipa::path(delete, path = "/widgets/{id}", responses((status = 409, description = "Conflict", body = FlatProblem)))]
#[allow(dead_code)]
fn delete_widget() {}

#[derive(OpenApi)]
#[openapi(paths(create_widget, get_widget, delete_widget))]
struct Api;

fn responses<T: IntoResponses>() -> Value {
    serde_json::to_value(T::responses()).unwrap()
}

fn titles(schema: &Value) -> Vec<String> {
    match schema.get("oneOf") {
        Some(Value::Array(items)) => items.iter().flat_map(titles).collect(),
        _ => vec![
            schema
                .get("title")
                .or_else(|| schema["allOf"][0].get("title"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        ],
    }
}

#[test]
fn set_responses_group_by_status_and_dedupe() {
    let responses = responses::<EndpointErrors>();
    let statuses: Vec<&String> = responses.as_object().unwrap().keys().collect();
    assert_eq!(statuses, ["400", "404", "409", "422", "429", "503"]);

    let schema_titles =
        |status: &str| titles(&responses[status]["content"]["application/problem+json"]["schema"]);
    // Variants first, then `documents`, depth-first; `UnitProblem` reached twice is documented once.
    assert_eq!(schema_titles("404"), ["UNIT"]);
    assert_eq!(schema_titles("400"), ["WrappedProblem"]);
    assert_eq!(schema_titles("409"), ["FlatProblem"]);
    assert_eq!(schema_titles("503"), ["ExplicitTitle"]);
    assert_eq!(responses["404"]["description"], "Not Found");
}

#[test]
fn statuses_shared_by_several_types_use_one_of() {
    #[problem_set(documents(WrappedProblem, ExternalProblem, NewtypeProblem))]
    #[derive(Debug)]
    enum BadRequests {}

    let responses = responses::<BadRequests>();
    let schema = &responses["400"]["content"]["application/problem+json"]["schema"];
    assert_eq!(titles(schema), ["WrappedProblem", "EXTERNAL", "NEWTYPE"]);
}

#[test]
fn single_problem_type_responses() {
    let responses = responses::<FlatProblem>();
    assert_eq!(
        responses.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["409"]
    );
    assert_eq!(
        responses["409"]["content"]["application/problem+json"]["schema"],
        serde_json::to_value(<FlatProblem as utoipa::PartialSchema>::schema()).unwrap()
    );
}

#[test]
fn body_registers_the_full_problem_schema() {
    let api = serde_json::to_value(Api::openapi()).unwrap();
    let component = &api["components"]["schemas"]["FlatProblem"];
    assert_eq!(
        component["properties"]["type"]["enum"],
        json!(["https://fixtures.test/problems/flat"])
    );
    assert_eq!(
        api["paths"]["/widgets/{id}"]["delete"]["responses"]["409"]["content"]["application/json"]
            ["schema"]["$ref"],
        "#/components/schemas/FlatProblem"
    );
}

#[test]
fn document_snapshot() {
    // `#[derive(OpenApi)]` copies the package metadata into `info` when the macro expands. Cargo does not rebuild
    // the test when that metadata changes, so the test sets a fixed `info`.
    let mut api = Api::openapi();
    api.info = utoipa::openapi::Info::new("problem-details-fine tests", "0.0.0");
    insta::assert_json_snapshot!(api);
}
