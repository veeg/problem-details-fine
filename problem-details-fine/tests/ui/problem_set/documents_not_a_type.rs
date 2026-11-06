use problem_details_fine::{problem_set, problem_type};

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct A;

// `documents` takes types.
#[problem_set(documents(1))]
enum Set {
    A(A),
}

fn main() {}
