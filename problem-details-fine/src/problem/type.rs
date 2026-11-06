use http::StatusCode;
use serde::{Serialize, Serializer};

/// Where the extension members of a problem type go in the problem details document.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extensions {
    /// The problem type has no extension members.
    None,
    /// The extension members go at the top level of the document.
    Flatten,
    /// The extension members go in the member with this name.
    Wrap(&'static str),
}

/// Whether the extension members of a problem type include a member with the name `tag`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagField {
    /// No extension member has the name `tag`.
    No,
    /// An extension member has the name `tag`.
    Yes,
    /// A `#[serde(flatten)]` field can add a member with the name `tag`.
    Unknown,
}

/// The standard members that the Openapi schema of a problem type leaves out, from `omit(..)` of `problem_type`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Omit {
    /// The schema has no `detail` property.
    pub detail: bool,
    /// The schema has no `instance` property.
    pub instance: bool,
}

/// The definition of one problem type.
///
/// The value of a problem type holds the extension members of its problem details document. Its `Serialize`
/// implementation writes the complete document in one pass. To change the document before you send it, convert
/// the value into a [`ProblemDetails`](crate::ProblemDetails).
///
/// # Implementing
///
/// Only the [`problem_type`](crate::problem_type) attribute macro implements this trait. See its documentation
/// for the full picture of how to work with `ProblemType`.
///
/// The trait also has a hidden
/// supertrait. It connects the serialization, the Openapi schema and the rewrite configuration of a problem type.
/// It is not part of the public API, and it can change in any release. Do not implement `ProblemType` by hand.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a problem type",
    label = "not a problem type",
    note = "define problem types with `#[problem_type(type = \"...\", status = ..., title = \"...\")]`"
)]
pub trait ProblemType: Serialize + ProblemTypeImpl + 'static {
    /// The type URI, as written in the definition. A rewrite does not change this value.
    const TYPE: &'static str;
    /// The tag URI of the problem type, as written in the definition. It is the `tag` argument, or
    /// [`ProblemType::TYPE`] if that is a tag URI. A rewrite does not change this value. For the tag URI after the
    /// rewrite, see [`ProblemDetails::tag_uri`](crate::ProblemDetails::tag_uri).
    const TAG: Option<&'static str>;
    /// The HTTP status code of the problem type.
    const STATUS: StatusCode;
    /// A short summary of the problem type, for a human reader. It is the same for each occurrence.
    const TITLE: &'static str;
    /// The title of the generated Openapi schema.
    const SCHEMA_TITLE: &'static str;

    /// An explanation of this occurrence of the problem, for a human reader.
    fn detail(&self) -> Option<String>;
}

/// The hidden items of a [`ProblemType`]. They are in a supertrait, so that an implementation of `ProblemType` by hand
/// fails with the message below, instead of a list of the missing hidden items.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` implements `ProblemType` by hand",
    label = "define this problem type with `#[problem_type(...)]`",
    note = "only the `#[problem_type]` attribute can implement `ProblemType`"
)]
pub trait ProblemTypeImpl {
    /// Where the extension members go in the document.
    const __EXTENSIONS: Extensions;

    /// Whether the extension members include a member with the name `tag`.
    const __TAG_FIELD: TagField;

    /// The standard members that the Openapi schema leaves out.
    const __OMIT: Omit;

    /// The path of the definition, `module::path::Ident`. It identifies the problem type in `const` evaluation.
    const __ID: &'static str;

    /// Serializes the extension members as a map, as `__EXTENSIONS` specifies.
    fn __serialize_extensions<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;

    /// The `type` and `tag` after the rewrite. The crate calculates them once for each problem type.
    fn __resolved() -> &'static crate::__private::Resolved;
}
