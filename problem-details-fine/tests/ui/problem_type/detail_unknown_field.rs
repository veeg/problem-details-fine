use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X", detail = "{missing}")]
struct Struct {
    present: String,
}

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X")]
#[serde(tag = "kind")]
enum Enum {
    #[problem(detail = "{nope}")]
    Variant { present: String },
}

#[problem_type(type = "tag:x.test,2025:C", status = NOT_FOUND, title = "X", detail = "{0}")]
struct Positional;

fn main() {}
