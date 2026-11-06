use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X", detail = "x", omit(detail))]
struct WithDetail;

fn describe(_: &WithDetailWith) -> Option<String> {
    None
}

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X", detail_with = describe, omit(detail))]
struct WithDetailWith;

#[problem_type(type = "tag:x.test,2025:C", status = NOT_FOUND, title = "X", omit(detail))]
#[serde(tag = "kind")]
enum WithVariantDetail {
    #[problem(detail = "x")]
    A,
    B,
}

#[problem_type(type = "tag:x.test,2025:D", status = NOT_FOUND, title = "X", omit(title))]
struct UnknownMember;

#[problem_type(type = "tag:x.test,2025:E", status = NOT_FOUND, title = "X", omit(detail, detail))]
struct Repeated;

fn main() {}
