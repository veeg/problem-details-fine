//! [RFC 9457] problem details documents, with problem types that you define once in code.
//!
//! - A [`ProblemType`] defines one kind of problem. Define it with [`problem_type`]. A problem type serializes as
//!   its complete problem details document.
//! - A [`ProblemSet`] is the set of problem types that an operation can return. Define it with [`problem_set`] on
//!   an enum that you write.
//! - [`ProblemDetails`] builds a problem details document. Use it for a problem without a problem type, or to
//!   change the document of a problem type before you send it.
//!
//! # Features
//!
//! | Feature | Effect |
//! |---|---|
//! | `utoipa` | Adds Openapi schemas and responses for problem types and problem sets. |
//! | `axum` | Adds `IntoResponse`. Responses use the media type `application/problem+json`. |
//! | `rewrite` | Adds the `rewrite` module: global rules that change type URIs and tag URIs. |
//! | `embedded-types` | Adds the `embedded` module with problem types for common HTTP API errors. |
//!
//! No feature is enabled by default.
//!
//! # Define a problem type
//!
//! ```rust
//! use problem_details_fine::{ProblemDetails, problem_type};
//!
//! #[problem_type(
//!     type = "https://example.com/problems/widget-conflict",
//!     status = CONFLICT,
//!     title = "A widget with that id already exists",
//!     detail = "widget `{id}` already exists",
//! )]
//! #[derive(Debug, Clone)]
//! struct WidgetConflict {
//!     id: String,
//! }
//!
//! let problem = WidgetConflict { id: "w-1".into() };
//! assert_eq!(
//!     serde_json::to_value(&problem).unwrap(),
//!     serde_json::json!({
//!         "type": "https://example.com/problems/widget-conflict",
//!         "status": 409,
//!         "title": "A widget with that id already exists",
//!         "detail": "widget `w-1` already exists",
//!         "id": "w-1",
//!     })
//! );
//!
//! let details = ProblemDetails::from(problem).with_instance("/widgets/w-1");
//! assert_eq!(details.type_uri(), "https://example.com/problems/widget-conflict");
//! ```
//!
//! The fields of a problem type are the extension members of its document. The macro puts them at the top level
//! of the document. With `wrap = "<member>"`, the macro puts them in one member instead.
//!
//! # Define a problem set
//!
//! ```rust
//! use problem_details_fine::{problem_set, problem_type};
//!
//! #[problem_type(type = "https://example.com/problems/not-found", status = NOT_FOUND, title = "Not found")]
//! #[derive(Debug)]
//! struct NotFound;
//!
//! #[problem_type(type = "https://example.com/problems/conflict", status = CONFLICT, title = "Conflict")]
//! #[derive(Debug)]
//! struct Conflict;
//!
//! /// Errors returned by `POST /widgets`.
//! #[problem_set]
//! #[derive(Debug)]
//! enum CreateWidgetError {
//!     NotFound(NotFound),
//!     Conflict(Conflict),
//! }
//!
//! fn create() -> Result<(), CreateWidgetError> {
//!     Err(Conflict)?
//! }
//! # assert!(create().is_err());
//! ```
//!
//! A handler can return a problem type or a problem set. With the `axum` feature, both implement `IntoResponse`.
//!
//! If the document does not serialize, the `axum` response has the status `500 Internal Server Error` and this
//! document. The response does not show the cause. Serialization fails if the extension members do not
//! serialize, or if the document has two `tag` members. See [Tag URIs](#tag-uris).
//!
//! ```json
//! {"type":"about:blank","status":500,"title":"Internal Server Error"}
//! ```
//!
//! # Openapi
//!
//! With the `utoipa` feature, problem types and problem sets implement `utoipa::IntoResponses`. Put them in
//! `responses(...)` of `#[utoipa::path]`. Each response schema describes the complete problem details document.
//!
//! A problem set documents each of its problem types. This includes the problem types in nested problem sets and
//! in `documents(...)`. The problem set gives one response for each status code. If two or more problem types have the same
//! status, the schema of that response is a `oneOf` of their schemas. See
//! [`problem_set`](crate::problem_set#openapi-responses).
//!
//! The responses of a problem set with each problem type of the `embedded` module follow, in the style of [Redoc].
//! Select a status code to see the schema of its response. If a response has more than one problem type, select
//! one under "One of".
//!
//! [Redoc]: https://github.com/Redocly/redoc
//!
// The `widgets` example generates `examples/widgets/responses.html`. Only rustdoc includes it, so that the compiled
// crate does not contain it.
#![cfg_attr(doc, doc = include_str!("../examples/widgets/responses.html"))]
//!
//! # Tag URIs
//!
//! A problem type can have a tag URI in addition to its type URI. The type URI can use any URI scheme. The tag
//! URI must be an [RFC 4151] tag URI: `tag:<authority>,<date>:<specific>`.
//!
//! The tag URI gives clients a value to match on that does not depend on where the documentation is. A type URI
//! such as `https://example.com/problems/widget-conflict` points at documentation, and it changes if the
//! documentation moves. The tag URI stays the same. With the `rewrite` feature, rewrite rules can change both
//! URIs. See [Rewrites](#rewrites).
//!
//! - If the type URI is a tag URI, it is also the tag URI of the problem type. The document has no `tag` member.
//! - Otherwise, give a tag URI with the `tag` argument of [`problem_type`]. The document then has the extension
//!   member `tag` with the tag URI.
//!
//! [`ProblemType::TAG`] returns the tag URI as written in the definition. [`ProblemDetails::tag_uri`] returns
//! the tag URI after the rewrite.
//!
//! ```rust
//! use problem_details_fine::problem_type;
//!
//! #[problem_type(
//!     type = "https://example.com/problems/widget-conflict",
//!     tag = "tag:example.com,2025:WIDGET_CONFLICT",
//!     status = CONFLICT,
//!     title = "A widget with that id already exists",
//! )]
//! #[derive(Debug)]
//! struct WidgetConflict;
//!
//! assert_eq!(
//!     serde_json::to_value(WidgetConflict).unwrap(),
//!     serde_json::json!({
//!         "type": "https://example.com/problems/widget-conflict",
//!         "tag": "tag:example.com,2025:WIDGET_CONFLICT",
//!         "status": 409,
//!         "title": "A widget with that id already exists",
//!     })
//! );
//! ```
//! A problem type can also have its own extension member `tag`. One document cannot have two `tag` members. A
//! collision occurs in these cases:
//!
//! - The `tag` argument and an extension member `tag`. The compiler gives an error.
//! - A problem type with a tag URI as its type URI, and an extension member `tag`. A rewrite rule changes the type
//!   URI to a URI that is not a tag URI, for example an `https` URI. The document then gets a `tag` member from
//!   the tag URI. Serialization fails, and the generation of the Openapi documentation panics. To prevent this,
//!   remove the tag URI in the rewrite rule with `rewrite::TagUri::Remove`.
//! - A [`ProblemDetails`] with a type URI that is not a tag URI, a tag URI from
//!   [`with_tag_uri`](ProblemDetails::with_tag_uri), and an extension member `tag`. Serialization fails.
//!
//! # Rewrites
//!
//! With the `rewrite` feature, global rules can change the type URIs and tag URIs that the crate emits. For
//! example, you can give the problem types in the `embedded` module URIs to your own documentation. A rewrite
//! changes the serialized documents and the Openapi schemas in the same way.
//!
// The `rewrite` module exists only with the `rewrite` feature. Thus, only then can the documentation link to it.
#![cfg_attr(
    feature = "rewrite",
    doc = "The [`rewrite`] module documents the rules."
)]
#![cfg_attr(
    not(feature = "rewrite"),
    doc = "The `rewrite` module documents the rules."
)]
//!
//! [RFC 9457]: https://www.rfc-editor.org/rfc/rfc9457.html
//! [RFC 4151]: https://www.rfc-editor.org/rfc/rfc4151.html

#![cfg_attr(docsrs, feature(doc_cfg))]

// The macros generate paths that start with `::problem_details_fine`. This line makes those paths work in this
// crate too.
extern crate self as problem_details_fine;

mod identity;
mod problem;
mod uri;

#[cfg(feature = "axum")]
mod axum;
#[cfg(feature = "utoipa")]
mod openapi;

#[cfg(feature = "embedded-types")]
pub mod embedded;
#[cfg(feature = "rewrite")]
pub mod rewrite;

#[doc(hidden)]
pub mod __private;

pub use problem::{ExtensionError, ProblemDetails, ProblemSet, ProblemType};

/// Defines a problem type on a struct or an enum.
///
/// On a struct, the fields are the extension members:
///
/// ```rust
/// use problem_details_fine::problem_type;
/// use serde_json::json;
///
/// /// A widget with the same id already exists.
/// #[problem_type(
///     type = "https://example.com/problems/widget-conflict",
///     status = CONFLICT,
///     title = "A widget with that id already exists",
///     detail = "widget `{id}` already exists",
/// )]
/// #[derive(Debug, Clone)]
/// struct WidgetConflict {
///     /// The conflicting id.
///     #[schema(example = "w-1")]
///     id: String,
/// }
///
/// assert_eq!(
///     serde_json::to_value(WidgetConflict { id: "w-1".into() }).unwrap(),
///     json!({
///         "type": "https://example.com/problems/widget-conflict",
///         "status": 409,
///         "title": "A widget with that id already exists",
///         "detail": "widget `w-1` already exists",
///         "id": "w-1",
///     })
/// );
/// ```
///
/// The value of the type holds the extension members of its problem details document.
///
/// ```rust
/// use problem_details_fine::problem_type;
/// use serde_json::json;
///
/// /// The widget in the request body is not valid.
/// #[problem_type(
///     type = "https://example.com/problems/invalid-widget",
///     status = UNPROCESSABLE_ENTITY,
///     title = "The widget is not valid",
///     detail = "compare the widget with the request body schema of `POST /widgets`",
/// )]
/// #[derive(Debug, Clone)]
/// #[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
/// enum InvalidWidget {
///     /// The name is empty.
///     EmptyName,
///     /// The name is longer than the limit.
///     #[problem(detail = "the name has {length} characters; the limit is {limit}")]
///     NameTooLong { length: usize, limit: usize },
/// }
///
/// assert_eq!(
///     serde_json::to_value(InvalidWidget::EmptyName).unwrap(),
///     json!({
///         "type": "https://example.com/problems/invalid-widget",
///         "status": 422,
///         "title": "The widget is not valid",
///         "detail": "compare the widget with the request body schema of `POST /widgets`",
///         "kind": "EMPTY_NAME",
///     })
/// );
/// assert_eq!(
///     serde_json::to_value(InvalidWidget::NameTooLong { length: 80, limit: 64 }).unwrap(),
///     json!({
///         "type": "https://example.com/problems/invalid-widget",
///         "status": 422,
///         "title": "The widget is not valid",
///         "detail": "the name has 80 characters; the limit is 64",
///         "kind": "NAME_TOO_LONG",
///         "length": 80,
///         "limit": 64,
///     })
/// );
/// ```
///
/// # Arguments
///
/// | Argument | Required | Value |
/// |---|---|---|
/// | `type = "<uri>"` | Yes | The type URI. It is an absolute URI, or an RFC 4151 tag URI `tag:<authority>,<date>:<specific>`. |
/// | `status = NOT_FOUND` or `status = 404` | Yes | A 4xx or 5xx status code: the name of an `http::StatusCode` constant, or a number. |
/// | `title = "..."` | Yes | A short summary of the problem type. It is the same for each occurrence. |
/// | `detail = "..."` | No | A format string for the `detail` member. It can use the fields, for example `"widget {id} exists"`. |
/// | `detail_with = path` | No | A `fn(&Self) -> Option<String>` for the `detail` member. Do not use it with `detail`. |
/// | `tag = "tag:..."` | No | A tag URI for the problem type, if `type` is not a tag URI. The document then has a `tag` member. |
/// | `wrap = "<member>"` | No | Puts the extension members in this one member. |
/// | `schema_title = "..."` | No | The title of the Openapi schema. See [Schema title](#schema-title) for the default. |
/// | `payload_schema = path` | No | A `fn() -> RefOr<Schema>` for the schema of the extension members. The macro then does not derive it. |
/// | `omit(detail, instance)` | No | Leaves `detail`, `instance` or both out of the Openapi schema. See [Omitted members](#omitted-members). |
///
/// #### Detail of an enum variant
///
/// On an enum, put `#[problem(detail = "...")]` on a variant to give that variant its own format string. The
/// format string can use the fields of the variant. The variant then does not use the `detail` argument. A
/// variant without the attribute uses the `detail` argument, or has no `detail` member if there is no `detail`
/// argument. The `InvalidWidget` example above uses both.
///
/// The macro rejects `#[problem(..)]` on a field, and `#[problem(detail = "...")]` together with `detail_with` or
/// `omit(detail)`.
///
/// #### Schema title
///
/// The schema title names the problem type in the Openapi document. It is the `title` of the schema, and tools
/// such as Redoc use it as the label of the problem type in a `oneOf`. The macro takes the first of these:
///
/// 1. The `schema_title` argument.
/// 2. The specific part of the tag URI: the text after `tag:<authority>,<date>:`. The tag URI is the `type`, or
///    the `tag` argument if `type` is not a tag URI.
/// 3. The name of the Rust type.
///
/// ```rust
/// use problem_details_fine::{ProblemType, problem_type};
///
/// // 1. The `schema_title` argument.
/// #[problem_type(type = "tag:example.com,2026:QUOTA", status = 429, title = "Quota", schema_title = "QuotaExceeded")]
/// struct Quota;
/// assert_eq!(Quota::SCHEMA_TITLE, "QuotaExceeded");
///
/// // 2. The specific part of the `type`, which is a tag URI.
/// #[problem_type(type = "tag:example.com,2026:WIDGET_LOCKED", status = 423, title = "Locked")]
/// struct Locked;
/// assert_eq!(Locked::SCHEMA_TITLE, "WIDGET_LOCKED");
///
/// // 2. The specific part of the `tag` argument.
/// #[problem_type(
///     type = "https://example.com/problems/widget-gone",
///     tag = "tag:example.com,2026:WIDGET_GONE",
///     status = 410,
///     title = "Gone",
/// )]
/// struct Gone;
/// assert_eq!(Gone::SCHEMA_TITLE, "WIDGET_GONE");
///
/// // 3. The name of the type, because there is no tag URI.
/// #[problem_type(type = "https://example.com/problems/widget-conflict", status = 409, title = "Conflict")]
/// struct WidgetConflict;
/// assert_eq!(WidgetConflict::SCHEMA_TITLE, "WidgetConflict");
/// ```
///
/// # Omitted members
///
/// The Openapi schema of a problem type has the optional members `detail` and `instance`, because code can set them
/// on the [`ProblemDetails`] of the problem. If your API never sends one of them for a problem type, leave it out of
/// the schema with `omit`:
///
/// ```
/// # use problem_details_fine::problem_type;
/// #[problem_type(
///     type = "https://example.com/problems/maintenance",
///     status = SERVICE_UNAVAILABLE,
///     title = "The service is down for maintenance",
///     omit(detail, instance),
/// )]
/// struct Maintenance;
/// ```
///
/// # Extension members
///
/// The macro serializes the value of the type, and adds the result to the document as its extension members:
///
/// - A unit struct has no extension members.
/// - The fields of a struct become members at the top level of the document.
/// - An enum with `#[serde(tag = "...")]` becomes members at the top level. Its variants can be unit variants
///   or have named fields. A tuple variant needs `#[serde(content = "...")]` as well, or `wrap`.
/// - A tuple struct, or an enum without `#[serde(tag = "...")]`, needs `wrap`.
/// - A type with `#[serde(transparent)]` needs both `wrap` and `payload_schema`.
/// - The macro rejects `#[serde(into = ..)]` and `#[serde(remote = ..)]`.
///
/// The serde attribute `#[serde(tag = "...")]` is not related to the `tag` argument of this macro.
///
/// An extension member cannot be named `type`, `status`, `title`, `detail` or `instance`. An extension member
/// can be named `tag`, unless the `tag` argument of this macro adds the `tag` member of the document. See
/// [Tag URIs](crate#tag-uris).
///
/// # Generated implementations
///
/// | Feature | Implementations for the type |
/// |---|---|
/// | Always | [`ProblemType`]. |
/// | Always | [`ProblemSet`], as a set with one problem type. |
/// | Always | `serde::Serialize`, which writes the complete problem details document in one pass. |
/// | Always | `From<Self>` for [`ProblemDetails`]. |
/// | `utoipa` | `utoipa::PartialSchema` |
/// | `utoipa` | `utoipa::ToSchema`, with the schema of the complete document. |
/// | `utoipa` | `utoipa::IntoResponses`, with one `application/problem+json` response for the status of the type. |
/// | `axum` | `axum::response::IntoResponse`, with the status of the type and the media type `application/problem+json`. |
///
/// Do not derive `Serialize` on the type. Keep your `#[serde(..)]` attributes: they control the extension
/// members.
///
/// `#[schema(..)]` attributes control the schema of the extension members. Without `utoipa`, they have no effect.
///
/// With `utoipa`, `responses(WidgetConflict)` and `responses((status = 409, body = WidgetConflict))` both
/// document the correct response. The first form takes the status from the type.
pub use problem_details_fine_macros::problem_type;

/// Defines a problem set on an enum. Each variant holds one problem type or one problem set.
///
/// ```rust
/// use problem_details_fine::{problem_set, problem_type};
///
/// #[problem_type(type = "https://example.com/problems/not-found", status = NOT_FOUND, title = "Not found")]
/// #[derive(Debug)]
/// struct NotFound;
///
/// #[problem_type(type = "https://example.com/problems/conflict", status = CONFLICT, title = "Conflict")]
/// #[derive(Debug)]
/// struct Conflict;
///
/// #[problem_type(type = "https://example.com/problems/bad-body", status = BAD_REQUEST, title = "Bad body")]
/// #[derive(Debug)]
/// struct BadBody;
///
/// /// Errors returned by `PUT /widgets/{id}`.
/// #[problem_set(documents(BadBody))]
/// #[derive(Debug)]
/// enum UpdateWidgetError {
///     NotFound(NotFound),
///     Conflict(Conflict),
/// }
///
/// fn update(exists: bool) -> Result<(), UpdateWidgetError> {
///     if !exists {
///         Err(NotFound)?;
///     }
///     Ok(())
/// }
/// # assert!(update(false).is_err());
/// ```
///
/// # Variants
///
/// The variants are the problems that the enum can hold. Each variant holds one [`ProblemSet`]: a problem type,
/// or a different problem set. The macro implements `From` for the type in each variant, so `?` converts a value
/// of that type into the enum. It does not implement `From` for the types inside a nested problem set.
///
/// A nested problem set is useful when several operations share it. For example, a helper function for
/// authentication can return its own problem set. Each handler then has one variant for that problem set.
///
/// # `documents(...)`
///
/// `documents(...)` lists problem types and problem sets that the operation can return, but that a different
/// part of the service makes. Examples are extractors and middleware. The macro adds them to the Openapi
/// documentation only (feature `utoipa`). The enum cannot hold them.
///
/// # Generated implementations
///
/// | Feature | Implementations for the enum |
/// |---|---|
/// | Always | [`ProblemSet`], where [`status`](ProblemSet::status) is the status of the active variant. |
/// | Always | `serde::Serialize`, which writes the complete document of the active variant. |
/// | Always | `From` for the type in each variant. |
/// | Always | `From<Self>` for [`ProblemDetails`]. |
/// | `utoipa` | `utoipa::IntoResponses`. See [Openapi responses](#openapi-responses). |
/// | `axum` | `axum::response::IntoResponse`, from the active variant. |
/// | `rewrite` | None. This feature changes the type URIs and tag URIs in the serialized document and the schema. |
///
/// # Openapi responses
///
/// With the `utoipa` feature, the enum implements `utoipa::IntoResponses`. Put the enum in `responses(...)` of
/// `#[utoipa::path]`. In this example, `Conflict` and `Locked` have the same status:
///
/// ```rust
/// use problem_details_fine::{problem_set, problem_type};
///
/// #[problem_type(type = "https://example.com/problems/not-found", status = NOT_FOUND, title = "Not found")]
/// #[derive(Debug)]
/// struct NotFound;
///
/// #[problem_type(type = "https://example.com/problems/conflict", status = CONFLICT, title = "Conflict")]
/// #[derive(Debug)]
/// struct Conflict;
///
/// #[problem_type(type = "https://example.com/problems/locked", status = CONFLICT, title = "Locked")]
/// #[derive(Debug)]
/// struct Locked;
///
/// /// Errors returned by `PUT /widgets/{id}`.
/// #[problem_set]
/// #[derive(Debug)]
/// enum UpdateWidgetError {
///     NotFound(NotFound),
///     Conflict(Conflict),
///     Locked(Locked),
/// }
///
/// # #[cfg(feature = "utoipa")]
/// #[utoipa::path(put, path = "/widgets/{id}", responses((status = 204, description = "Updated"), UpdateWidgetError))]
/// fn update_widget() {}
/// # #[cfg(feature = "utoipa")]
/// # {
/// # use utoipa::IntoResponses;
/// # let responses = serde_json::to_value(UpdateWidgetError::responses()).unwrap();
/// # assert_eq!(responses.as_object().unwrap().keys().collect::<Vec<_>>(), ["404", "409"]);
/// # let one_of = &responses["409"]["content"]["application/problem+json"]["schema"]["oneOf"];
/// # assert_eq!(one_of.as_array().unwrap().len(), 2);
/// # }
/// ```
///
/// The responses contain each problem type of the set. This includes the problem types in nested problem sets and
/// in `documents(...)`.
///
/// - There is one `application/problem+json` response for each status code. Its description is the reason phrase
///   of the status, for example `Conflict`.
/// - If one problem type has the status, the response schema is the schema of its complete document.
/// - If two or more problem types have the status, the response schema is a `oneOf` of their schemas. In the
///   example, the `409` response is a `oneOf` of the schemas of `Conflict` and `Locked`.
/// - A problem type that occurs more than once in the set occurs once in the responses.
///
/// The `oneOf` has no `discriminator`. The `type` member is required, and the schema of each problem type allows
/// only its own type URI. Thus, exactly one schema of the `oneOf` matches a document. A client can read the `type`
/// member to find the problem type.
///
/// A problem set does not implement `utoipa::ToSchema`, because it can have more than one status. Do not use it as
/// a `body`.
///
/// # Distinct type URIs
///
/// Each problem type in a set must have a different type URI. This includes problem types in nested problem sets
/// and in `documents`. The same problem type can occur more than once.
///
/// If two problem types have the same type URI, the compiler gives an error. The error points at the variant or the
/// `documents` entry that adds the second problem type. It shows the URI, the two problem types, and the members of
/// the set that hold them.
///
/// The rewrite configuration can also give two problem types the same type URI. The crate finds this only when
/// it generates the Openapi documentation, and then it panics. A test that builds your `utoipa::OpenApi`
/// document finds this problem. See [`ProblemSet`](ProblemSet#distinct-type-uris).
pub use problem_details_fine_macros::problem_set;

/// The media type of a problem details document.
pub const MEDIA_TYPE: &str = "application/problem+json";
