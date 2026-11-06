use problem_details_fine::problem_type;

#[problem_type(type = "https://x.test/a", tag = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct Field {
    tag: String,
}

#[problem_type(type = "https://x.test/b", tag = "tag:x.test,2025:B", status = NOT_FOUND, title = "X")]
struct Renamed {
    #[serde(rename = "tag")]
    label: String,
}

#[problem_type(type = "https://x.test/c", tag = "tag:x.test,2025:C", status = NOT_FOUND, title = "X", wrap = "tag")]
struct Wrapped {
    value: String,
}

#[problem_type(type = "https://x.test/d", tag = "tag:x.test,2025:D", status = NOT_FOUND, title = "X")]
#[serde(tag = "tag")]
enum EnumTag {
    A,
}

fn main() {}
