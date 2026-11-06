use problem_details_fine::problem_type;

#[problem_type(type = "/problems/not-found", status = NOT_FOUND, title = "X")]
struct Relative;

#[problem_type(type = "about:blank", status = NOT_FOUND, title = "X")]
struct AboutBlank;

fn main() {}
