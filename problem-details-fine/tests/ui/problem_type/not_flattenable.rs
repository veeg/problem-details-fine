use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct Tuple(String);

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X")]
enum Untagged {
    A,
}

#[problem_type(type = "tag:x.test,2025:C", status = NOT_FOUND, title = "X", wrap = "value")]
struct NothingToWrap;

fn main() {}
