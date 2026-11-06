use problem_details_fine::problem_type;

#[problem_type(type = "tag:x.test,2025:A", status = NOT_FOUND, title = "X")]
#[serde(transparent)]
struct Transparent {
    inner: u32,
}

#[problem_type(type = "tag:x.test,2025:E", status = NOT_FOUND, title = "X", wrap = "value")]
#[serde(transparent)]
struct TransparentWithoutSchema {
    inner: u32,
}

#[problem_type(type = "tag:x.test,2025:B", status = NOT_FOUND, title = "X")]
#[serde(tag = "kind")]
enum TaggedNewtype {
    A(u32),
}

#[problem_type(type = "tag:x.test,2025:C", status = NOT_FOUND, title = "X")]
#[serde(tag = "kind", content = "detail")]
enum ReservedContent {
    A { x: u32 },
}

#[problem_type(type = "tag:x.test,2025:D", status = NOT_FOUND, title = "X")]
struct CfgAttrRename {
    #[cfg_attr(all(), serde(rename = "status"))]
    code: u16,
}

#[problem_type(type = "tag:x.test,2025:F", status = NOT_FOUND, title = "X", wrap = "value")]
#[serde(into = "String")]
#[derive(Clone)]
struct Into {
    inner: u32,
}

fn main() {}
