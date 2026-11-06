//! `#[problem_set]`: construction, conversion and delegation.

use crate::types::{
    ComputedProblem, ExplicitTagProblem, FlatProblem, TaggedProblem, UnitProblem, WrappedProblem,
};
use problem_details_fine::{ProblemDetails, problem_set};
use serde_json::json;

/// Errors from authentication.
#[problem_set]
#[derive(Debug)]
pub enum AuthErrors {
    Missing(UnitProblem),
    Throttled(ComputedProblem),
}

/// Errors produced before a handler runs, overlapping with `AuthErrors`.
#[problem_set]
#[derive(Debug)]
pub enum CommonErrors {
    Missing(UnitProblem),
    Wrapped(WrappedProblem),
}

/// Errors from an endpoint, including a nested set.
#[problem_set(documents(ExplicitTagProblem, CommonErrors))]
#[derive(Debug)]
pub enum EndpointErrors {
    /// A conflict.
    Conflict(FlatProblem),
    Tagged(TaggedProblem),
    Auth(AuthErrors),
}

/// Only documentation.
#[problem_set(documents(UnitProblem))]
#[derive(Debug)]
pub enum Documented {}

fn endpoint(fail: u8) -> Result<(), EndpointErrors> {
    match fail {
        0 => Ok(()),
        1 => Err(FlatProblem {
            widget_id: "w".into(),
            count: 1,
            note: None,
        })?,
        2 => Err(TaggedProblem::Plain)?,
        _ => Err(AuthErrors::from(UnitProblem))?,
    }
}

#[test]
fn question_mark_converts_variants() {
    assert!(endpoint(0).is_ok());
    assert!(matches!(endpoint(1), Err(EndpointErrors::Conflict(_))));
    assert!(matches!(
        endpoint(2),
        Err(EndpointErrors::Tagged(TaggedProblem::Plain))
    ));
    assert!(matches!(
        endpoint(3),
        Err(EndpointErrors::Auth(AuthErrors::Missing(UnitProblem)))
    ));
}

#[test]
fn converts_through_the_active_variant() {
    let details = ProblemDetails::from(endpoint(1).unwrap_err());
    assert_eq!(details.type_uri(), "https://fixtures.test/problems/flat");
    assert_eq!(details.status(), http::StatusCode::CONFLICT);
    assert_eq!(details.detail(), Some("widget `w` exists 1 times"));
    assert_eq!(details.extensions()["widgetId"], "w");

    let nested = ProblemDetails::from(EndpointErrors::from(AuthErrors::from(ComputedProblem {
        retry_after: 1,
    })));
    assert_eq!(nested.type_uri(), "tag:fixtures.test,2025:COMPUTED");
    assert_eq!(nested.detail(), Some("retry in 1s"));
}

#[test]
fn serializes_as_active_variant() {
    let expected = json!({
        "type": "tag:fixtures.test,2025:TAGGED",
        "status": 422,
        "title": "Tagged",
        "detail": "plain",
        "kind": "PLAIN",
    });
    assert_eq!(
        serde_json::to_value(endpoint(2).unwrap_err()).unwrap(),
        expected
    );
    assert_eq!(
        serde_json::to_value(ProblemDetails::from(endpoint(2).unwrap_err())).unwrap(),
        expected
    );

    let nested = EndpointErrors::from(AuthErrors::from(UnitProblem));
    assert_eq!(
        serde_json::to_value(nested).unwrap(),
        serde_json::to_value(UnitProblem).unwrap()
    );
}

#[test]
fn documentation_only_set_is_uninhabited() {
    fn _never(documented: Documented) -> ! {
        match documented {}
    }
}

#[problem_set]
#[derive(Debug)]
pub enum ConfiguredSet {
    Present(UnitProblem),
    #[cfg(any())]
    Absent(FlatProblem),
}

#[test]
fn configured_out_variants_are_skipped() {
    let set = ConfiguredSet::from(UnitProblem);
    assert_eq!(
        ProblemDetails::from(set).type_uri(),
        "tag:fixtures.test,2025:UNIT"
    );
}

#[test]
fn status_of_the_active_variant() {
    use problem_details_fine::ProblemSet;

    assert_eq!(
        endpoint(1).unwrap_err().status(),
        http::StatusCode::CONFLICT
    );
    assert_eq!(
        endpoint(3).unwrap_err().status(),
        http::StatusCode::NOT_FOUND
    );
    assert_eq!(
        ProblemSet::status(&UnitProblem),
        http::StatusCode::NOT_FOUND
    );
}

#[test]
fn problem_sets_and_types_share_one_bound() {
    fn render(problem: impl problem_details_fine::ProblemSet) -> (u16, String) {
        let status = problem.status().as_u16();
        let details: ProblemDetails = problem.into();
        (status, details.type_uri().into_owned())
    }

    assert_eq!(
        render(UnitProblem),
        (404, "tag:fixtures.test,2025:UNIT".to_owned())
    );
    assert_eq!(
        render(endpoint(2).unwrap_err()),
        (422, "tag:fixtures.test,2025:TAGGED".to_owned())
    );
}
