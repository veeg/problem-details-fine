use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
struct Field {
    title: String,
}

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X")]
struct Renamed {
    #[serde(rename = "type")]
    kind: String,
}

#[problem_type(type = "tag:x.test,2025:C", status = NOT_FOUND, title = "X")]
#[serde(rename_all = "kebab-case")]
struct RawIdentifier {
    r#type: String,
}

#[problem_type(type = "tag:x.test,2025:D", status = NOT_FOUND, title = "X")]
#[serde(tag = "detail")]
enum EnumTag {
    A,
}

#[problem_type(type = "tag:x.test,2025:E", status = NOT_FOUND, title = "X", wrap = "instance")]
struct Wrap {
    value: String,
}

fn main() {}
