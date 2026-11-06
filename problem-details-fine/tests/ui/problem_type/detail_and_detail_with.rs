use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X", detail = "x", detail_with = describe)]
struct Both;

fn describe(_: &Both) -> Option<String> {
    None
}

fn main() {}
