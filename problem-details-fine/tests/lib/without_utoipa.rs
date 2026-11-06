//! Without the `utoipa` feature, `#[schema(..)]` attributes are removed and `documents` has no effect.

use problem_details_fine::{problem_set, problem_type};

#[problem_type(type = "tag:x.test,2025:DOCUMENTED", status = BAD_REQUEST, title = "Documented")]
#[derive(Debug)]
#[schema(title = "Ignored")]
struct DocumentedProblem {
    #[schema(example = "ignored")]
    field: String,
}

#[problem_set(documents(DocumentedProblem))]
#[derive(Debug)]
enum OnlyDocumented {}

#[test]
fn compiles_and_serializes() {
    let _ = std::marker::PhantomData::<OnlyDocumented>;
    let value = serde_json::to_value(DocumentedProblem { field: "f".into() }).unwrap();
    assert_eq!(value["field"], "f");
}
