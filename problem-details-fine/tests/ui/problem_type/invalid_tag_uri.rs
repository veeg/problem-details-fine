use problem_details_fine::problem_type;

#[problem_type(type = "tag:api.example.com:2025:NOT_FOUND", status = NOT_FOUND, title = "X")]
struct InvalidTagType;

#[problem_type(type = "https://x.test/a", tag = "tag:example.com,25:A", status = NOT_FOUND, title = "X")]
struct InvalidTag;

fn main() {}
