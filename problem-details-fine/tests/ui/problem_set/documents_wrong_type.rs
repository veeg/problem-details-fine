use problem_details_fine::{problem_set, problem_type};

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct A;

// A type that holds a problem type is not a problem type.
#[problem_set(documents(Vec<A>))]
enum Set {
    A(A),
}

fn main() {}
