use http::StatusCode;
use problem_details_fine::{ProblemDetails, ProblemSet};

#[derive(serde::Serialize)]
struct Manual;

impl From<Manual> for ProblemDetails {
    fn from(_: Manual) -> Self {
        unimplemented!()
    }
}

impl ProblemSet for Manual {
    fn status(&self) -> StatusCode {
        StatusCode::NOT_FOUND
    }
}

fn main() {}
