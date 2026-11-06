//! `#[problem_type]`: trait constants, details and serialized documents.

use crate::types::{
    AdjacentProblem, ComputedProblem, ConfiguredProblem, ExplicitTagProblem, ExternalProblem,
    FlatProblem, NewtypeProblem, OwnTagProblem, RawProblem, TagUriOwnTagProblem, TaggedProblem,
    TransparentProblem, UnitProblem, WrappedProblem,
};
use problem_details_fine::{
    __private::{Extensions, ProblemTypeImpl},
    ProblemDetails, ProblemType,
};
use serde_json::{Value, json};

/// The serialized document, checking that serializing directly and through `ProblemDetails` agree.
fn document<P: ProblemType>(problem: P) -> Value {
    let direct = serde_json::to_value(&problem).unwrap();
    assert_eq!(
        serde_json::to_value(ProblemDetails::from(problem)).unwrap(),
        direct
    );
    direct
}

#[test]
fn constants() {
    assert_eq!(UnitProblem::TYPE, "tag:fixtures.test,2025:UNIT");
    assert_eq!(UnitProblem::TAG, Some("tag:fixtures.test,2025:UNIT"));
    assert_eq!(UnitProblem::STATUS, http::StatusCode::NOT_FOUND);
    assert_eq!(UnitProblem::TITLE, "Unit");
    assert_eq!(UnitProblem::SCHEMA_TITLE, "UNIT");
    assert_eq!(UnitProblem::__EXTENSIONS, Extensions::None);

    assert_eq!(FlatProblem::TAG, None);
    assert_eq!(FlatProblem::SCHEMA_TITLE, "FlatProblem");
    assert_eq!(FlatProblem::__EXTENSIONS, Extensions::Flatten);

    assert_eq!(WrappedProblem::__EXTENSIONS, Extensions::Wrap("error"));
    assert_eq!(TaggedProblem::STATUS.as_u16(), 422);
    assert_eq!(TaggedProblem::__EXTENSIONS, Extensions::Flatten);

    assert_eq!(
        ExplicitTagProblem::TAG,
        Some("tag:fixtures.test,2025:EXPLICIT")
    );
    assert_eq!(ExplicitTagProblem::SCHEMA_TITLE, "ExplicitTitle");
    assert_eq!(ExplicitTagProblem::__EXTENSIONS, Extensions::None);
}

#[test]
fn unit() {
    assert_eq!(
        document(UnitProblem),
        json!({"type": "tag:fixtures.test,2025:UNIT", "status": 404, "title": "Unit"})
    );
}

#[test]
fn flattened_struct_with_format_detail() {
    let flat = FlatProblem {
        widget_id: "w-1".into(),
        count: 2,
        note: None,
    };
    assert_eq!(
        flat.detail().as_deref(),
        Some("widget `w-1` exists 2 times")
    );
    assert_eq!(
        document(flat),
        json!({
            "type": "https://fixtures.test/problems/flat",
            "status": 409,
            "title": "Flat",
            "detail": "widget `w-1` exists 2 times",
            "widgetId": "w-1",
            "count": 2,
        })
    );
}

#[test]
fn wrapped_struct() {
    assert_eq!(
        document(WrappedProblem { kind: "k".into() }),
        json!({
            "type": "https://fixtures.test/problems/wrapped",
            "status": 400,
            "title": "Wrapped",
            "error": {"type": "k"},
        })
    );
}

#[test]
fn tagged_enum_details() {
    assert_eq!(TaggedProblem::Plain.detail().as_deref(), Some("plain"));
    assert_eq!(
        TaggedProblem::Field {
            name: "n".into(),
            value: 3
        }
        .detail()
        .as_deref(),
        Some(r#"field "n" is 3"#)
    );
    assert_eq!(
        TaggedProblem::Fallback { thing: "x".into() }
            .detail()
            .as_deref(),
        Some("something about the x")
    );
}

#[test]
fn tagged_enum_flattened() {
    assert_eq!(
        document(TaggedProblem::Field {
            name: "n".into(),
            value: 3
        }),
        json!({
            "type": "tag:fixtures.test,2025:TAGGED",
            "status": 422,
            "title": "Tagged",
            "detail": r#"field "n" is 3"#,
            "kind": "FIELD",
            "name": "n",
            "value": 3,
        })
    );
    assert_eq!(document(TaggedProblem::Plain)["kind"], "PLAIN");
}

#[test]
fn wrapped_enum_and_newtype() {
    assert_eq!(document(ExternalProblem::Simple)["reason"], json!("Simple"));
    assert_eq!(
        document(ExternalProblem::Complex { depth: 2 })["reason"],
        json!({"Complex": {"depth": 2}})
    );
    assert_eq!(ExternalProblem::Simple.detail(), None);
    assert_eq!(document(NewtypeProblem(7))["value"], json!(7));
}

#[test]
fn detail_with() {
    assert_eq!(
        document(ComputedProblem { retry_after: 0 }).get("detail"),
        None
    );
    assert_eq!(
        document(ComputedProblem { retry_after: 3 })["detail"],
        "retry in 3s"
    );
}

#[test]
fn skipped_and_raw_fields() {
    assert_eq!(
        document(RawProblem {
            r#loop: "l".into(),
            title: "ignored".into()
        }),
        json!({"type": "tag:fixtures.test,2025:RAW", "status": 400, "title": "Raw", "detail": "loop l", "loop": "l"})
    );
}

#[test]
fn details_overrides() {
    let details = ProblemDetails::from(ComputedProblem { retry_after: 3 })
        .with_detail("custom")
        .with_instance("/requests/1")
        .with_extension("extra", true);
    assert_eq!(details.status(), http::StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(details.title(), "Computed");
    assert_eq!(details.type_uri(), "tag:fixtures.test,2025:COMPUTED");
    assert_eq!(details.extensions()["retry_after"], 3);
    let value = serde_json::to_value(&details).unwrap();
    assert_eq!(value["detail"], "custom");
    assert_eq!(value["instance"], "/requests/1");
    assert_eq!(value["extra"], true);
    assert_eq!(value["retry_after"], 3);
}

#[test]
fn direct_serialization_member_order() {
    let flat = FlatProblem {
        widget_id: "w".into(),
        count: 1,
        note: None,
    };
    assert_eq!(
        serde_json::to_string(&flat).unwrap(),
        r#"{"type":"https://fixtures.test/problems/flat","status":409,"title":"Flat","detail":"widget `w` exists 1 times","widgetId":"w","count":1}"#
    );
}

#[test]
fn adjacently_tagged_enum() {
    assert_eq!(
        document(AdjacentProblem::Titled { title: "t".into() }),
        json!({
            "type": "tag:fixtures.test,2025:ADJACENT",
            "status": 400,
            "title": "Adjacent",
            "kind": "Titled",
            "data": {"title": "t"},
        })
    );
    assert_eq!(document(AdjacentProblem::Count(2))["data"], 2);
}

#[test]
fn configured_variants_and_fields() {
    assert_eq!(
        document(ConfiguredProblem::Present { x: 1 }),
        json!({
            "type": "tag:fixtures.test,2025:CONFIGURED",
            "status": 400,
            "title": "Configured",
            "detail": "1",
            "kind": "Present",
            "renamed": 1,
        })
    );
}

#[test]
fn transparent_wrapped() {
    assert_eq!(document(TransparentProblem { inner: 3 })["value"], 3);
}

#[test]
fn own_tag_member_without_a_tag_uri_member() {
    assert_eq!(
        document(OwnTagProblem { tag: "mine".into() }),
        json!({
            "type": "https://fixtures.test/problems/own-tag",
            "status": 400,
            "title": "Own tag",
            "tag": "mine",
        })
    );
    // The type URI is a tag URI, thus the document has no `tag` member for the tag URI.
    assert_eq!(
        document(TagUriOwnTagProblem { tag: "mine".into() })["tag"],
        "mine"
    );
}
