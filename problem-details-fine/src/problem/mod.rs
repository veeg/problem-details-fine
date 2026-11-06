//! The definitions of problem types, problem sets and the problem details document.

mod details;
mod set;
mod r#type;

#[cfg(feature = "utoipa")]
pub(crate) use details::tag_collision;
pub use details::{ExtensionError, ProblemDetails};
pub use details::{serialize_no_extensions, serialize_problem, serialize_wrapped};
pub use set::{ProblemSet, ProblemSetImpl};
pub use set::{assert_distinct_members, copy_types};
pub use r#type::{Extensions, Omit, ProblemType, ProblemTypeImpl, TagField};
