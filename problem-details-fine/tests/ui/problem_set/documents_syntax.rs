use problem_details_fine::{problem_set, problem_type};

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct A;

// `documents` takes a list in parentheses.
#[problem_set(documents = A)]
enum Set {
    A(A),
}

fn main() {}
