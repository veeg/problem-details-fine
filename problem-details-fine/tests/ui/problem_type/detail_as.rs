use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X", detail_as = "error")]
struct NoDetailSource;

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X", detail = "x", detail_as = "error", omit(detail))]
struct WithOmitDetail;

#[problem_type(type = "tag:x.test,2025:C", status = NOT_FOUND, title = "X", detail = "x", detail_as = "title")]
struct Reserved;

#[problem_type(type = "tag:x.test,2025:D", status = NOT_FOUND, title = "X", detail = "x", detail_as = "id")]
struct FieldCollision {
    id: String,
}

#[problem_type(type = "tag:x.test,2025:E", status = NOT_FOUND, title = "X", detail = "x", detail_as = "kind")]
#[serde(tag = "kind")]
enum SerdeTagCollision {
    A,
}

#[problem_type(type = "tag:x.test,2025:F", status = NOT_FOUND, title = "X", detail = "x", detail_as = "data", wrap = "data")]
struct WrapCollision {
    id: String,
}

#[problem_type(
    type = "https://x.test/problems/g",
    tag = "tag:x.test,2025:G",
    status = NOT_FOUND,
    title = "X",
    detail = "x",
    detail_as = "tag",
)]
struct TagArgumentCollision;

#[problem_type(type = "tag:x.test,2025:H", status = NOT_FOUND, title = "X", detail = "x", detail_as = "")]
struct Empty;

#[problem_type(type = "tag:x.test,2025:I", status = NOT_FOUND, title = "X", detail = "x", detail_as = "a", detail_as = "b")]
struct Repeated;

fn main() {}
