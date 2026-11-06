use problem_details_fine::{problem_set, problem_type};

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct A;

#[problem_type(type = "tag:x.test,2025:B", status = CONFLICT, title = "X")]
struct B;

#[problem_set(documents(B))]
enum Set {
    A(A),
}

fn handler() -> Result<(), Set> {
    Err(B)?
}

fn main() {}
