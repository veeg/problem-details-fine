//! Items for the code that `problem_type` and `problem_set` generate. This module is not public API.

pub use http;
pub use serde;

#[cfg(feature = "axum")]
pub use axum;
#[cfg(feature = "utoipa")]
pub use utoipa;

pub use problem_details_fine_macros::__ProblemHelpers;

#[cfg(feature = "axum")]
pub use crate::axum::into_response;
pub use crate::identity::Resolved;
#[cfg(feature = "utoipa")]
pub use crate::openapi::{ProblemEntry, problem_responses, problem_schema};
pub use crate::problem::{
    Extensions, Omit, ProblemSetImpl, ProblemTypeImpl, TagField, assert_distinct_members,
    copy_types, serialize_no_extensions, serialize_problem, serialize_wrapped,
};

/// Returns the status code for an integer literal. The macro validates the literal.
pub const fn status(code: u16) -> http::StatusCode {
    match http::StatusCode::from_u16(code) {
        Ok(status) => status,
        Err(_) => panic!("invalid status code"),
    }
}

/// Emits its input if `problem-details-fine` has the `utoipa` feature.
#[cfg(feature = "utoipa")]
#[macro_export]
#[doc(hidden)]
macro_rules! __if_utoipa {
    ($($tokens:tt)*) => { $($tokens)* };
}

/// Emits its input if `problem-details-fine` has the `utoipa` feature.
#[cfg(not(feature = "utoipa"))]
#[macro_export]
#[doc(hidden)]
macro_rules! __if_utoipa {
    ($($tokens:tt)*) => {};
}

/// Emits its input if `problem-details-fine` has the `axum` feature.
#[cfg(feature = "axum")]
#[macro_export]
#[doc(hidden)]
macro_rules! __if_axum {
    ($($tokens:tt)*) => { $($tokens)* };
}

/// Emits its input if `problem-details-fine` has the `axum` feature.
#[cfg(not(feature = "axum"))]
#[macro_export]
#[doc(hidden)]
macro_rules! __if_axum {
    ($($tokens:tt)*) => {};
}

/// Adds the `ToSchema` derive to the payload twin of a problem type, if `problem-details-fine` has the `utoipa`
/// feature. The payload twin is the hidden copy of the problem type that the macro generates.
#[cfg(feature = "utoipa")]
#[macro_export]
#[doc(hidden)]
macro_rules! __problem_payload {
    ($($item:tt)*) => {
        // The `ToSchema` derive generates paths that start with `utoipa::`.
        use ::problem_details_fine::__private::utoipa;
        #[derive(utoipa::ToSchema)]
        $($item)*
    };
}

/// Adds the `ToSchema` derive to the payload twin of a problem type, if `problem-details-fine` has the `utoipa`
/// feature. The payload twin is the hidden copy of the problem type that the macro generates.
#[cfg(not(feature = "utoipa"))]
#[macro_export]
#[doc(hidden)]
macro_rules! __problem_payload {
    ($($item:tt)*) => { $($item)* };
}
