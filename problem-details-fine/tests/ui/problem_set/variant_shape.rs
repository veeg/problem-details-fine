use problem_details_fine::{problem_set, problem_type};

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct A;

#[problem_set]
enum Set {
    Unit,
    Named { a: A },
    Two(A, A),
}

fn main() {}
