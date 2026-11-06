//! The integration tests, in one test binary.
//!
//! `tests/rewrite.rs` is the only other test binary, because it registers a global rewrite configuration.

mod types;

#[path = "lib/axum.rs"]
#[cfg(feature = "axum")]
mod axum;
#[cfg(all(feature = "embedded-types", feature = "utoipa"))]
#[path = "lib/embedded.rs"]
mod embedded;
#[cfg(feature = "utoipa")]
#[path = "lib/omit.rs"]
mod omit;
#[cfg(feature = "utoipa")]
#[path = "openapi/mod.rs"]
mod openapi;
#[path = "lib/problem_set.rs"]
mod problem_set;
#[path = "lib/problem_type.rs"]
mod problem_type;
#[cfg(feature = "rewrite")]
#[path = "lib/rewrite.rs"]
mod rewrite;
#[path = "lib/tag.rs"]
mod tag;
// The expected compiler output depends on the features. Thus, the compile-fail tests run only with exactly the
// `utoipa` feature.
#[cfg(all(
    feature = "utoipa",
    not(any(feature = "axum", feature = "rewrite", feature = "embedded-types"))
))]
#[path = "ui/ui.rs"]
mod ui;
#[cfg(not(feature = "utoipa"))]
#[path = "lib/without_utoipa.rs"]
mod without_utoipa;
