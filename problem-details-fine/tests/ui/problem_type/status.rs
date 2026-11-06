use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = 302, title = "X")]
struct Redirect;

#[problem_type(type = "tag:x.test,2025:B", status = OK, title = "X")]
struct Ok;

#[problem_type(type = "tag:x.test,2025:C", status = NOT_A_STATUS, title = "X")]
struct Unknown;

fn main() {}
