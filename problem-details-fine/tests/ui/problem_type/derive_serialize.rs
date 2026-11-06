use problem_details_fine::problem_type;

// `problem_type` implements `Serialize`, so a derived `Serialize` conflicts with it.

// The derive after the attribute.
#[problem_type(type = "tag:x.test,2025:AFTER", status = NOT_FOUND, title = "X")]
#[derive(serde::Serialize)]
struct After;

// The derive before the attribute. `problem_type` does not see this derive, so the compiler reports the conflict.
#[derive(serde::Serialize)]
#[problem_type(type = "tag:x.test,2025:BEFORE", status = NOT_FOUND, title = "X")]
struct Before;

// With other derives, `#[serde(..)]` attributes and extension members.
#[problem_type(type = "tag:x.test,2025:MEMBERS", status = NOT_FOUND, title = "X")]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Members {
    user_id: u32,
}

// With `wrap`.
#[problem_type(type = "tag:x.test,2025:WRAPPED", status = NOT_FOUND, title = "X", wrap = "error")]
#[derive(serde::Serialize)]
struct Wrapped(u32);

fn main() {}
