use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct Generic<T> {
    value: T,
}

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X")]
union Union {
    a: u32,
}

fn main() {}
