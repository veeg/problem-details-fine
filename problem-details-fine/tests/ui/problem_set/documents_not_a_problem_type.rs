use problem_details_fine::{problem_set, problem_type};

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct A;

#[derive(serde::Serialize)]
struct Plain;

#[problem_set(documents(Plain))]
enum Set {
    A(A),
}

fn main() {}
