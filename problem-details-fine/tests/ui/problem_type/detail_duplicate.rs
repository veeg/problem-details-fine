use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X", detail_duplicate = "message")]
struct NoDetailSource;

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X", detail = "x", detail_duplicate(detail))]
struct Reserved;

#[problem_type(type = "tag:x.test,2025:C", status = NOT_FOUND, title = "X", detail = "x", detail_duplicate(id))]
struct FieldCollision {
    id: String,
}

#[problem_type(type = "tag:x.test,2025:D", status = NOT_FOUND, title = "X", detail = "x", detail_as = "error", detail_duplicate = "message")]
struct WithDetailAs;

#[problem_type(
    type = "tag:x.test,2025:E",
    status = NOT_FOUND,
    title = "X",
    detail = "x",
    detail_duplicate = "code",
    fixed(/** The code. */ code: &str = "E"),
)]
struct FixedCollision;

#[problem_type(
    type = "https://x.test/problems/f",
    tag = "tag:x.test,2025:F",
    status = NOT_FOUND,
    title = "X",
    detail = "x",
    detail_duplicate = "tag",
)]
struct TagArgumentCollision;

#[problem_type(type = "tag:x.test,2025:G", status = NOT_FOUND, title = "X", detail = "x", detail_duplicate = "")]
struct Empty;

#[problem_type(type = "tag:x.test,2025:H", status = NOT_FOUND, title = "X", detail = "x", detail_duplicate = "a", detail_duplicate = "b")]
struct Repeated;

#[problem_type(type = "tag:x.test,2025:I", status = NOT_FOUND, title = "X", detail = "x", detail_duplicate(a, b))]
struct TwoNames;

#[problem_type(type = "tag:x.test,2025:J", status = NOT_FOUND, title = "X", detail = "x", detail_duplicate(#[serde] a))]
struct NotADocComment;

fn main() {}
