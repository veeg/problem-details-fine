use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", tag = "tag:x.test,2025:B", status = NOT_FOUND, title = "X")]
struct Redundant;

fn main() {}
