use problem_details_fine::{problem_set, problem_type};

#[problem_type(type = "tag:x.test,2025:SAME", status = NOT_FOUND, title = "A")]
struct A;

#[problem_type(type = "tag:x.test,2025:SAME", status = CONFLICT, title = "B")]
struct B;

#[problem_type(type = "tag:x.test,2025:OTHER", status = CONFLICT, title = "C")]
struct C;

// Direct: two variants.
#[problem_set]
enum Direct {
    A(A),
    B(B),
}

// Nested: `A` through a nested set, `B` through `documents`.
#[problem_set]
enum Inner {
    A(A),
    C(C),
}

#[problem_set(documents(B))]
enum Nested {
    Inner(Inner),
}

// The same type reached twice is fine.
#[problem_set(documents(Inner))]
enum Reached {
    A(A),
}

fn main() {}
