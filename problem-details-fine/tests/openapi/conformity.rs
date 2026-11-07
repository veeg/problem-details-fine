//! Every serialized problem document validates against the problem type's own `ToSchema` schema.

use crate::types::{
    AdjacentProblem, ComputedProblem, ConfiguredProblem, ExplicitTagProblem, ExternalProblem,
    FlatProblem, Location, ManualProblem, NewtypeProblem, RawProblem, ReferencingProblem,
    RetrofitAdjacentProblem, RetrofitEnumProblem, RetrofitPartialProblem, RetrofitProblem,
    RetrofitUnitProblem, RetrofitWrappedProblem, TaggedProblem, TransparentProblem, UnitProblem,
    WrappedProblem,
};
use problem_details_fine::{ProblemDetails, ProblemType};
use serde_json::{Value, json};
use utoipa::PartialSchema;

fn schema_of<T: PartialSchema>() -> Value {
    serde_json::to_value(T::schema()).unwrap()
}

#[track_caller]
fn assert_conforms<T: ProblemType + PartialSchema>(problem: T) {
    let schema = schema_of::<T>();
    let direct = serde_json::to_value(&problem).unwrap();
    let document =
        serde_json::to_value(ProblemDetails::from(problem).with_instance("/requests/1")).unwrap();
    assert!(
        jsonschema::is_valid(&schema, &direct),
        "direct serialization does not conform: {direct:#}"
    );
    let validator = jsonschema::validator_for(&schema).expect("valid JSON schema");
    let errors: Vec<String> = validator
        .iter_errors(&document)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "document does not conform:\n{errors:#?}\ndocument: {document:#}\nschema: {schema:#}"
    );
}

#[test]
fn fixtures_conform() {
    assert_conforms(UnitProblem);
    assert_conforms(FlatProblem {
        widget_id: "w".into(),
        count: 1,
        note: None,
    });
    assert_conforms(FlatProblem {
        widget_id: "w".into(),
        count: 1,
        note: Some("n".into()),
    });
    assert_conforms(WrappedProblem { kind: "k".into() });
    assert_conforms(TaggedProblem::Plain);
    assert_conforms(TaggedProblem::Field {
        name: "n".into(),
        value: -1,
    });
    assert_conforms(TaggedProblem::Fallback { thing: "t".into() });
    assert_conforms(ExternalProblem::Simple);
    assert_conforms(ExternalProblem::Complex { depth: 1 });
    assert_conforms(NewtypeProblem(1));
    assert_conforms(ComputedProblem { retry_after: 2 });
    assert_conforms(ExplicitTagProblem);
    assert_conforms(ReferencingProblem {
        location: Location { line: 1, column: 2 },
        locations: vec![Location { line: 3, column: 4 }],
    });
    assert_conforms(ManualProblem {
        opaque: json!({"anything": [1, 2]}),
    });
    assert_conforms(AdjacentProblem::Titled { title: "t".into() });
    assert_conforms(AdjacentProblem::Count(1));
    assert_conforms(ConfiguredProblem::Present { x: 1 });
    assert_conforms(TransparentProblem { inner: 1 });
    assert_conforms(RawProblem {
        r#loop: "l".into(),
        title: String::new(),
    });
}

/// The `f32` fixed member has the same JSON number in the document and in the schema's `enum`.
#[test]
fn f32_fixed_conforms() {
    assert_conforms(RetrofitProblem { id: "w-1".into() });
}

/// Each of these types with `detail_as` and `fixed(...)` conforms to its own schema:
///
/// - An internally tagged enum where every variant renders a detail.
/// - An internally tagged enum where a variant has no format string.
/// - A `wrap` type.
/// - A type with no fields.
/// - An adjacently tagged enum.
#[test]
fn retrofit_shapes_conform() {
    assert_conforms(RetrofitEnumProblem::Missing { id: "a".into() });
    assert_conforms(RetrofitEnumProblem::Invalid { id: "b".into() });
    assert_conforms(RetrofitPartialProblem::Missing { id: "c".into() });
    assert_conforms(RetrofitPartialProblem::Other { id: "d".into() });
    assert_conforms(RetrofitWrappedProblem { id: "w".into() });
    assert_conforms(RetrofitUnitProblem);
    assert_conforms(RetrofitAdjacentProblem::Titled { title: "t".into() });
    assert_conforms(RetrofitAdjacentProblem::Count(1));
}

#[test]
fn validation_is_not_vacuous() {
    let schema = schema_of::<FlatProblem>();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let valid = json!({"type": "https://fixtures.test/problems/flat", "status": 409, "title": "Flat", "widgetId": "w", "count": 1});
    assert!(validator.is_valid(&valid));

    for (member, value) in [
        ("type", json!("https://fixtures.test/problems/other")),
        ("status", json!(400)),
        ("count", json!("one")),
    ] {
        let mut invalid = valid.clone();
        invalid[member] = value;
        assert!(!validator.is_valid(&invalid), "{member} should be rejected");
    }
    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("widgetId");
    assert!(!validator.is_valid(&missing));
}

#[test]
fn flattened_object_payload_is_merged() {
    let schema = schema_of::<FlatProblem>();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["title"], "FlatProblem");
    assert_eq!(
        schema["required"],
        json!(["type", "status", "title", "widgetId", "count"])
    );
    assert_eq!(schema["properties"]["widgetId"]["example"], "w-1");
    assert_eq!(
        schema["properties"]["widgetId"]["description"],
        "The widget id."
    );
    assert!(schema.get("allOf").is_none());
}

#[test]
fn flattened_enum_payload_uses_all_of() {
    let schema = schema_of::<TaggedProblem>();
    let all_of = schema["allOf"].as_array().expect("allOf");
    assert_eq!(all_of.len(), 2);
    assert_eq!(all_of[0]["title"], "TAGGED");
    assert!(all_of[1]["oneOf"].is_array());
}

#[test]
fn wrapped_payload_is_a_required_member() {
    let schema = schema_of::<WrappedProblem>();
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("error"))
    );
    assert_eq!(
        schema["properties"]["error"]["properties"]["type"]["type"],
        "string"
    );
}

#[test]
fn referenced_schemas_are_inlined() {
    let schema = schema_of::<ReferencingProblem>().to_string();
    assert!(!schema.contains("$ref"), "{schema}");
}
