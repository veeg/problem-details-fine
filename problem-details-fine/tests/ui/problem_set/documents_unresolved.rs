use problem_details_fine::{problem_set, problem_type};

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct A;

// `Missing` is not defined.
#[problem_set(documents(Missing))]
enum Set {
    A(A),
}

fn main() {}
