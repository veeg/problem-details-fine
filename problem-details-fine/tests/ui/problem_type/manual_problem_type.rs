use http::StatusCode;
use problem_details_fine::ProblemType;

#[derive(serde::Serialize)]
struct Manual;

impl ProblemType for Manual {
    const TYPE: &'static str = "tag:x.test,2025:MANUAL";
    const TAG: Option<&'static str> = Some(Self::TYPE);
    const STATUS: StatusCode = StatusCode::NOT_FOUND;
    const TITLE: &'static str = "Manual";
    const SCHEMA_TITLE: &'static str = "MANUAL";

    fn detail(&self) -> Option<String> {
        None
    }
}

fn main() {}
