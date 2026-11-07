use problem_details_fine::problem_type;

const CODE: &str = "X";

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X", fixed(/** Doc. */ status: u16 = 404))]
struct Reserved;

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X", fixed(/** Doc. */ code: &str = "A", /** Doc. */ "code": &str = "B"))]
struct Duplicate;

#[problem_type(type = "tag:x.test,2025:C", status = NOT_FOUND, title = "X", fixed(/** Doc. */ id: &str = "A"))]
struct FieldCollision {
    id: String,
}

#[problem_type(type = "tag:x.test,2025:D", status = NOT_FOUND, title = "X", fixed(/** Doc. */ widgetId: &str = "A"))]
#[serde(rename_all = "camelCase")]
struct RenamedFieldCollision {
    widget_id: String,
}

#[problem_type(type = "tag:x.test,2025:E", status = NOT_FOUND, title = "X", wrap = "data", fixed(/** Doc. */ data: &str = "A"))]
struct WrapCollision {
    id: String,
}

#[problem_type(type = "tag:x.test,2025:F", status = NOT_FOUND, title = "X", fixed(/** Doc. */ kind: &str = "A"))]
#[serde(tag = "kind")]
enum SerdeTagCollision {
    A,
}

#[problem_type(type = "tag:x.test,2025:G", status = NOT_FOUND, title = "X", fixed(/** Doc. */ c: &str = "A"))]
#[serde(tag = "t", content = "c")]
enum SerdeContentCollision {
    A(u32),
}

#[problem_type(type = "tag:x.test,2025:H", status = NOT_FOUND, title = "X", detail = "x", detail_as = "error", fixed(/** Doc. */ error: &str = "A"))]
struct DetailAsCollision;

#[problem_type(
    type = "https://x.test/problems/i",
    tag = "tag:x.test,2025:I",
    status = NOT_FOUND,
    title = "X",
    fixed(/** Doc. */ tag: &str = "A"),
)]
struct TagArgumentCollision;

#[problem_type(type = "tag:x.test,2025:J", status = NOT_FOUND, title = "X", fixed(code: &str = "A"))]
struct NoDoc;

#[problem_type(type = "tag:x.test,2025:K", status = NOT_FOUND, title = "X", fixed(/** Doc. */ code: &str = CODE))]
struct NotLiteral;

#[problem_type(type = "tag:x.test,2025:L", status = NOT_FOUND, title = "X", fixed(/** Doc. */ code: &str "A"))]
struct MissingEquals;

#[problem_type(type = "tag:x.test,2025:M", status = NOT_FOUND, title = "X", fixed(/** Doc. */ code: = "A"))]
struct MissingType;

#[problem_type(type = "tag:x.test,2025:N", status = NOT_FOUND, title = "X", fixed(/** Doc. */ 5: &str = "A"))]
struct NumberName;

#[problem_type(type = "tag:x.test,2025:O", status = NOT_FOUND, title = "X", fixed(/** Doc. */ "": &str = "A"))]
struct EmptyName;

#[problem_type(type = "tag:x.test,2025:P", status = NOT_FOUND, title = "X", fixed(/** Doc. */ a: &str = "A"), fixed(/** Doc. */ b: &str = "B"))]
struct Repeated;

fn main() {}
