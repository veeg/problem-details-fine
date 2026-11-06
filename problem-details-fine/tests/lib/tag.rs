//! The `tag` member, emitted only when `type` is not itself a tag URI.

use crate::types::{ExplicitTagProblem, FlatProblem, UnitProblem, WrappedProblem};
use problem_details_fine::ProblemDetails;
use serde_json::json;

#[test]
fn tag_member_only_for_non_tag_types() {
    assert_eq!(
        serde_json::to_value(ExplicitTagProblem).unwrap(),
        json!({
            "type": "https://fixtures.test/problems/explicit-tag",
            "tag": "tag:fixtures.test,2025:EXPLICIT",
            "status": 503,
            "title": "Explicit",
        })
    );
    assert_eq!(serde_json::to_value(UnitProblem).unwrap().get("tag"), None);
    assert_eq!(
        serde_json::to_value(WrappedProblem { kind: "k".into() })
            .unwrap()
            .get("tag"),
        None
    );
}

#[test]
fn member_order() {
    assert_eq!(
        serde_json::to_string(&ProblemDetails::from(ExplicitTagProblem).with_detail("d")).unwrap(),
        r#"{"type":"https://fixtures.test/problems/explicit-tag","tag":"tag:fixtures.test,2025:EXPLICIT","status":503,"title":"Explicit","detail":"d"}"#
    );
}

#[cfg(feature = "utoipa")]
#[test]
fn tag_in_schema() {
    use utoipa::PartialSchema;
    let schema = serde_json::to_value(ExplicitTagProblem::schema()).unwrap();
    assert_eq!(
        schema["properties"]["tag"]["enum"],
        json!(["tag:fixtures.test,2025:EXPLICIT"])
    );
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("tag"))
    );

    let schema = serde_json::to_value(UnitProblem::schema()).unwrap();
    assert!(schema["properties"].get("tag").is_none());
}

#[test]
fn tag_accessor_is_the_identity() {
    assert_eq!(
        ProblemDetails::from(UnitProblem).tag_uri().as_deref(),
        Some("tag:fixtures.test,2025:UNIT")
    );
    assert_eq!(
        ProblemDetails::from(ExplicitTagProblem)
            .tag_uri()
            .as_deref(),
        Some("tag:fixtures.test,2025:EXPLICIT")
    );
    assert_eq!(
        ProblemDetails::from(FlatProblem {
            widget_id: "w".into(),
            count: 1,
            note: None
        })
        .tag_uri(),
        None
    );
}
