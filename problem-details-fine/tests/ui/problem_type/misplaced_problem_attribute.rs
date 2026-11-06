use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X", detail_with = describe)]
#[serde(tag = "kind")]
enum WithDetailWith {
    #[problem(detail = "a")]
    A,
}

fn describe(_: &WithDetailWith) -> Option<String> {
    None
}

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X")]
struct OnField {
    #[problem(detail = "x")]
    x: u32,
}

#[problem_type(type = "tag:x.test,2025:C", status = NOT_FOUND, title = "X", detail = "{x:>w$}")]
struct ArgumentWidth {
    x: u32,
}

fn main() {}
