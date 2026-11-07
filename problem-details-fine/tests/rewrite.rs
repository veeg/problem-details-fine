#![cfg(feature = "rewrite")]
//! The global rewrite configuration changes serialized documents and schemas in the same way.
//!
//! The configuration is global to the process. Thus, this file is a separate test binary, and all its tests use the
//! one configuration from `setup`.

mod types;

use std::sync::{Mutex, Once};

use problem_details_fine::{
    ProblemDetails, ProblemType,
    rewrite::{RewriteConfig, RewriteError, RewriteRule, TagUri},
};
use serde_json::Value;
use types::{
    ComputedProblem, ExplicitTagProblem, FlatProblem, FlattenedTagProblem, Labels, RawProblem,
    RetrofitProblem, TagUriOwnTagProblem, TaggedProblem, UnitProblem, WrappedProblem,
};

/// The type URIs that `on_unmatched` gets.
static UNMATCHED: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn setup() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        RewriteConfig::new()
            .rule(RewriteRule::substring(
                "fixtures.test,2025",
                "example.com,2026",
            ))
            .rule(
                RewriteRule::exact(UnitProblem::TYPE)
                    .type_uri("https://docs.example.com/problems/unit"),
            )
            .rule(RewriteRule::exact(FlatProblem::TYPE).type_uri("https://docs.example.com/flat"))
            .rule(
                RewriteRule::exact(ExplicitTagProblem::TYPE)
                    .type_uri("https://docs.example.com/explicit")
                    .tag_uri(TagUri::Remove),
            )
            .rule(
                RewriteRule::exact(ComputedProblem::TYPE)
                    .type_uri("https://docs.example.com/computed")
                    .tag_uri(TagUri::replace("tag:example.com,2026:REPLACED")),
            )
            .rule(
                RewriteRule::exact(WrappedProblem::TYPE)
                    .tag_uri(TagUri::replace("tag:example.com,2026:WRAPPED")),
            )
            .rule(RewriteRule::ignore("tag:fixtures.test,2025:RAW"))
            .rule(
                RewriteRule::exact(TagUriOwnTagProblem::TYPE)
                    .type_uri("https://docs.example.com/own-tag"),
            )
            .rule(
                RewriteRule::exact(FlattenedTagProblem::TYPE)
                    .type_uri("https://docs.example.com/flattened-tag"),
            )
            .rule(
                RewriteRule::exact("tag:fixtures.test,2025:TAG_URI_OWN_TAG_REMOVED")
                    .type_uri("https://docs.example.com/own-tag-removed")
                    .tag_uri(TagUri::Remove),
            )
            .rule(
                RewriteRule::exact("tag:fixtures.test,2025:FIXED_TAG")
                    .type_uri("https://docs.example.com/fixed-tag"),
            )
            .rule(
                RewriteRule::exact("https://rewrite.test/collision-a")
                    .type_uri("https://docs.example.com/same"),
            )
            .rule(
                RewriteRule::exact("https://rewrite.test/collision-b")
                    .type_uri("https://docs.example.com/same"),
            )
            .on_unmatched(|unmatched| {
                UNMATCHED
                    .lock()
                    .unwrap()
                    .push(unmatched.type_uri().to_owned())
            })
            .register_global()
            .unwrap();
    });
}

/// A problem type that the tests can serialize, convert and, with `utoipa`, describe.
#[cfg(feature = "utoipa")]
trait Fixture: ProblemType + Clone + utoipa::PartialSchema {}
#[cfg(feature = "utoipa")]
impl<T: ProblemType + Clone + utoipa::PartialSchema> Fixture for T {}
#[cfg(not(feature = "utoipa"))]
trait Fixture: ProblemType + Clone {}
#[cfg(not(feature = "utoipa"))]
impl<T: ProblemType + Clone> Fixture for T {}

/// The serialized document. Also checks that direct serialization, `ProblemDetails` and the schema agree.
fn document<P: Fixture>(problem: P) -> Value {
    setup();

    let direct = serde_json::to_value(&problem).unwrap();
    let details = ProblemDetails::from(problem.clone());
    assert_eq!(serde_json::to_value(&details).unwrap(), direct);
    assert_eq!(direct["type"], details.type_uri().as_ref());

    #[cfg(feature = "utoipa")]
    {
        let schema = serde_json::to_value(<P as utoipa::PartialSchema>::schema()).unwrap();
        assert!(
            jsonschema::is_valid(&schema, &direct),
            "{direct:#} against {schema:#}"
        );
        let (properties, required) = match schema.get("allOf") {
            Some(all_of) => (&all_of[0]["properties"], &all_of[0]["required"]),
            None => (&schema["properties"], &schema["required"]),
        };
        assert_eq!(properties["type"]["enum"][0], direct["type"]);
        match direct.get("tag") {
            Some(tag) => {
                assert_eq!(&properties["tag"]["enum"][0], tag);
                assert!(required.as_array().unwrap().contains(&Value::from("tag")));
            }
            None => assert!(
                properties.get("tag").is_none(),
                "the schema has a `tag` property"
            ),
        }
    }

    direct
}

#[test]
fn register_global_is_once() {
    setup();
    assert_eq!(
        RewriteConfig::new().register_global(),
        Err(RewriteError::AlreadyRegistered)
    );
}

#[test]
fn exact_rule_keeps_the_tag() {
    let unit = document(UnitProblem);
    assert_eq!(unit["type"], "https://docs.example.com/problems/unit");
    assert_eq!(unit["tag"], "tag:example.com,2026:UNIT");
    assert_eq!(
        ProblemDetails::from(UnitProblem).tag_uri().as_deref(),
        Some("tag:example.com,2026:UNIT")
    );
    assert_eq!(unit["title"], "Unit");
}

#[test]
fn exact_rule_without_a_tag() {
    let flat = document(FlatProblem {
        widget_id: "w".into(),
        count: 1,
        note: None,
    });
    assert_eq!(flat["type"], "https://docs.example.com/flat");
    assert_eq!(flat.get("tag"), None);
}

#[test]
fn exact_rule_removes_the_tag() {
    let explicit = document(ExplicitTagProblem);
    assert_eq!(explicit["type"], "https://docs.example.com/explicit");
    assert_eq!(explicit.get("tag"), None);
    assert_eq!(ProblemDetails::from(ExplicitTagProblem).tag_uri(), None);
}

#[test]
fn exact_rule_replaces_the_tag() {
    let computed = document(ComputedProblem { retry_after: 1 });
    assert_eq!(computed["type"], "https://docs.example.com/computed");
    assert_eq!(computed["tag"], "tag:example.com,2026:REPLACED");
}

#[test]
fn exact_rule_adds_a_tag_without_changing_the_type() {
    let wrapped = document(WrappedProblem { kind: "k".into() });
    assert_eq!(wrapped["type"], "https://fixtures.test/problems/wrapped");
    assert_eq!(wrapped["tag"], "tag:example.com,2026:WRAPPED");
}

#[test]
fn substring_rule() {
    let tagged = document(TaggedProblem::Plain);
    assert_eq!(tagged["type"], "tag:example.com,2026:TAGGED");
    assert_eq!(tagged.get("tag"), None);
}

#[test]
fn ignore_rule() {
    let raw = document(RawProblem {
        r#loop: "l".into(),
        title: String::new(),
    });
    assert_eq!(raw["type"], "tag:fixtures.test,2025:RAW");
}

#[test]
fn rewrite_leaves_retrofit_members() {
    let retrofit = document(RetrofitProblem { id: "w-1".into() });
    assert_eq!(retrofit["type"], "tag:example.com,2026:RETROFIT");
    assert_eq!(retrofit["error"], "widget `w-1` already exists");
    assert_eq!(retrofit["code"], "RETROFIT");
    assert_eq!(retrofit["ratio"], 0.1);
}

#[test]
fn rewrite_that_adds_a_tag_member_to_a_fixed_tag_member_fails() {
    #[problem_details_fine::problem_type(
        type = "tag:fixtures.test,2025:FIXED_TAG",
        status = BAD_REQUEST,
        title = "Fixed tag",
        fixed(
            /// A legacy member named `tag`.
            tag: &str = "mine",
        ),
    )]
    #[derive(Debug, Clone)]
    struct FixedTagProblem;

    setup();
    for error in [
        serde_json::to_string(&FixedTagProblem).unwrap_err(),
        serde_json::to_string(&ProblemDetails::from(FixedTagProblem)).unwrap_err(),
    ] {
        assert!(
            error.to_string().contains("also an extension member `tag`"),
            "{error}"
        );
    }
}

#[test]
fn about_blank_is_untouched() {
    setup();
    let value = serde_json::to_value(ProblemDetails::new(http::StatusCode::NOT_FOUND)).unwrap();
    assert_eq!(value["type"], "about:blank");
}

#[test]
fn ad_hoc_documents_are_rewritten() {
    setup();
    let details = ProblemDetails::new(http::StatusCode::CONFLICT).with_type_uri(FlatProblem::TYPE);
    assert_eq!(details.type_uri(), "https://docs.example.com/flat");
}

#[test]
fn rewrite_that_adds_a_tag_member_to_an_own_tag_member_fails() {
    setup();
    for error in [
        serde_json::to_string(&TagUriOwnTagProblem { tag: "mine".into() }).unwrap_err(),
        serde_json::to_string(&ProblemDetails::from(TagUriOwnTagProblem {
            tag: "mine".into(),
        }))
        .unwrap_err(),
        serde_json::to_string(&FlattenedTagProblem {
            labels: Labels { tag: "mine".into() },
        })
        .unwrap_err(),
        serde_json::to_string(&ProblemDetails::from(FlattenedTagProblem {
            labels: Labels { tag: "mine".into() },
        }))
        .unwrap_err(),
    ] {
        assert!(
            error.to_string().contains("also an extension member `tag`"),
            "{error}"
        );
    }
}

#[test]
fn rewrite_that_removes_the_tag_uri_keeps_the_own_tag_member() {
    #[problem_details_fine::problem_type(
        type = "tag:fixtures.test,2025:TAG_URI_OWN_TAG_REMOVED",
        status = BAD_REQUEST,
        title = "Removed"
    )]
    #[derive(Debug, Clone)]
    struct RemovedTagProblem {
        tag: String,
    }

    setup();
    let problem = RemovedTagProblem { tag: "mine".into() };
    let removed = serde_json::to_value(&problem).unwrap();
    assert_eq!(
        serde_json::to_value(ProblemDetails::from(problem)).unwrap(),
        removed
    );
    assert_eq!(removed["type"], "https://docs.example.com/own-tag-removed");
    assert_eq!(removed["tag"], "mine");
}

#[cfg(feature = "utoipa")]
mod openapi {
    use problem_details_fine::{ProblemType, problem_set, problem_type};
    use utoipa::IntoResponses;

    use super::{UNMATCHED, setup};
    use crate::types::{
        FlattenedTagProblem, RawProblem, TagUriOwnTagProblem, TaggedProblem, UnitProblem,
    };

    /// No rule matches these problem types.
    #[problem_type(type = "https://rewrite.test/unmatched-a", status = BAD_REQUEST, title = "Unmatched A")]
    #[derive(Debug, Clone)]
    struct UnmatchedAProblem;

    #[problem_type(type = "https://rewrite.test/unmatched-b", status = CONFLICT, title = "Unmatched B")]
    #[derive(Debug, Clone)]
    struct UnmatchedBProblem;

    /// Two exact rules give these problem types the same type URI.
    #[problem_type(type = "https://rewrite.test/collision-a", status = BAD_REQUEST, title = "Collision A")]
    #[derive(Debug, Clone)]
    struct CollisionAProblem;

    #[problem_type(type = "https://rewrite.test/collision-b", status = BAD_REQUEST, title = "Collision B")]
    #[derive(Debug, Clone)]
    struct CollisionBProblem;

    #[problem_set(documents(UnmatchedBProblem, TaggedProblem))]
    #[derive(Debug)]
    enum Endpoint {
        Unit(UnitProblem),
        UnmatchedA(UnmatchedAProblem),
        Raw(RawProblem),
    }

    #[problem_set]
    #[derive(Debug)]
    enum Collides {
        A(CollisionAProblem),
        B(CollisionBProblem),
    }

    #[test]
    fn on_unmatched_gets_each_problem_type_without_a_rule_once() {
        setup();
        let _ = Endpoint::responses();
        let _ = Endpoint::responses();
        // Other tests in this binary can add other type URIs, for example `about:blank`.
        let unmatched = UNMATCHED.lock().unwrap().clone();
        for type_uri in [UnmatchedAProblem::TYPE, UnmatchedBProblem::TYPE] {
            assert_eq!(
                unmatched.iter().filter(|seen| *seen == type_uri).count(),
                1,
                "{type_uri} in {unmatched:?}"
            );
        }
        for type_uri in [UnitProblem::TYPE, RawProblem::TYPE, TaggedProblem::TYPE] {
            assert!(
                !unmatched.iter().any(|seen| seen == type_uri),
                "{type_uri} in {unmatched:?}"
            );
        }
    }

    #[test]
    #[should_panic(expected = "also an extension member `tag`")]
    fn schema_of_a_rewrite_that_adds_a_tag_member_to_an_own_tag_member_panics() {
        setup();
        let _ = <TagUriOwnTagProblem as utoipa::PartialSchema>::schema();
    }

    #[test]
    #[should_panic(expected = "also an extension member `tag`")]
    fn schema_of_a_rewrite_that_adds_a_tag_member_to_a_flattened_tag_panics() {
        setup();
        let _ = <FlattenedTagProblem as utoipa::PartialSchema>::schema();
    }

    #[test]
    #[should_panic(expected = "both present the type URI `https://docs.example.com/same`")]
    fn two_problem_types_with_the_same_type_uri_panic() {
        setup();
        let _ = Collides::responses();
    }
}
