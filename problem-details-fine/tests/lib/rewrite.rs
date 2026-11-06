//! The first read fixes the configuration. Thus, documents and schemas cannot disagree.

use problem_details_fine::{
    ProblemDetails,
    rewrite::{RewriteConfig, RewriteError},
};

#[test]
fn register_after_first_read_fails() {
    // A read of the `type` member reads the configuration.
    let _ = ProblemDetails::new(http::StatusCode::NOT_FOUND).type_uri();
    assert_eq!(
        RewriteConfig::new().register_global(),
        Err(RewriteError::AlreadyRegistered)
    );
}
