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
//! # Retrofit
//!
//! An API can already have an error format of its own, for example
//! `{ "error": "...", "code": "WIDGET_CONFLICT", "success": false }`. With `detail_as`, `detail_duplicate` and
//! `fixed(...)`, the API can send problem details documents that keep each member its clients read. `detail_as` writes
//! the rendered detail to the extension member that you name, instead of `detail`. `detail_duplicate` writes it to a
//! second member and keeps `detail`. `fixed(...)` adds extension members with a constant value, such as a legacy `code`
//! that identifies the problem type.
//!
//! ```rust
//! use problem_details_fine::problem_type;
//! use serde_json::json;
//!
//! #[problem_type(
//!     type = "https://example.com/problems/widget-conflict",
//!     status = CONFLICT,
//!     title = "A widget with that id already exists",
//!     detail = "widget `{id}` already exists",
//!     detail_as = "error",
//!     fixed(
//!         /// The legacy error code. It does not change between releases.
//!         code: &str = "WIDGET_CONFLICT",
//!         /// Always `false` for errors.
//!         success: bool = false,
//!     ),
//! )]
//! #[derive(Debug)]
//! struct WidgetConflict {
//!     id: String,
//! }
//!
//! assert_eq!(
//!     serde_json::to_value(WidgetConflict { id: "w-1".into() }).unwrap(),
//!     json!({
//!         "type": "https://example.com/problems/widget-conflict",
//!         "status": 409,
//!         "title": "A widget with that id already exists",
//!         "error": "widget `w-1` already exists",
//!         "code": "WIDGET_CONFLICT",
//!         "success": false,
//!         "id": "w-1",
//!     })
//! );
//! ```
//!
//! The document has no `detail` member. The `type`, `status` and `title` members do not change. See
//! [Retrofit](problem_type#retrofit) in the `problem_type` documentation for the rules.
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
/// | `detail_as = "<member>"` | No | Writes the detail as this extension member instead of `detail`. |
/// | `detail_duplicate = "<member>"` | No | Also writes the detail as this extension member. A doc comment is optional. See [Retrofit](#retrofit). |
/// | `fixed(...)` | No | Extension members with a constant value. See [Retrofit](#retrofit). |
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
/// # Retrofit
///
/// `detail_as`, `detail_duplicate` and `fixed(...)` add the members of an existing error format to a problem type. The document is then
/// valid problem details, and it still has the members that a client reads. [Retrofit](crate#retrofit) in the crate
/// documentation has an example.
///
/// ## `detail_as`
///
/// `detail = "..."`, `detail_with` and the variant attribute `#[problem(detail = "...")]` render the detail string
/// exactly as they do without `detail_as`. With `detail_as = "<member>"`, the macro writes that string to the named
/// extension member instead of `detail`. The document has no `detail` member. If the rendered value is `None`, the
/// document also has no `detail_as` member.
///
/// [`ProblemType::detail`] still returns the rendered string. A caller can use it to log the problem. The
/// [`ProblemDetails`] from [`ProblemDetails::from`] has the string in `extensions()`, under the `detail_as` name. Its
/// `detail()` returns `None`. [`ProblemDetails::with_detail`] on that value adds a standard `detail` member next to
/// the `detail_as` member. The two members are independent.
///
/// The Openapi schema leaves out `detail` and adds the `detail_as` member as a `string` property. The property is
/// required if each value of the type has a format string. These types have one:
///
/// - A struct with the `detail` argument.
/// - An enum with the `detail` argument.
/// - An enum where each variant has `#[problem(detail = "...")]`.
///
/// The property is optional with `detail_with`, which returns `Option<String>`. It is also optional for an enum where
/// a variant has no format string.
///
/// `detail_as` needs a detail source: the `detail` argument, `detail_with`, or `#[problem(detail = "...")]` on at
/// least one variant. The macro rejects `detail_as` together with `omit(detail)`, because `detail_as` already leaves
/// `detail` out of the schema.
///
/// ## `detail_duplicate`
///
/// `detail_duplicate` writes the rendered detail to a second member. The document keeps the `detail` member. Use it
/// while clients move from a legacy member, such as `message`, to `detail`.
///
/// ```rust
/// use problem_details_fine::problem_type;
/// use serde_json::json;
///
/// #[problem_type(
///     type = "https://example.com/problems/widget-conflict",
///     status = CONFLICT,
///     title = "A widget with that id already exists",
///     detail = "widget `{id}` already exists",
///     detail_duplicate(
///         /// The legacy error message. It has the same value as `detail`.
///         message
///     ),
/// )]
/// #[derive(Debug)]
/// struct WidgetConflict {
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
///         "message": "widget `w-1` already exists",
///         "id": "w-1",
///     })
/// );
/// ```
///
/// The argument has two forms:
///
/// - `detail_duplicate = "<member>"` gives only the name.
/// - `detail_duplicate(<member>)` can also have a doc comment before the name. The name is a Rust identifier or a
///   string literal, as in `fixed(...)`.
///
/// The Openapi schema adds the member as a `string` property. The doc comment is the `description` of the property.
/// Without a doc comment, the description is the same as the description of `detail`. The property is required under
/// the same rules as the `detail_as` property.
///
/// If the rendered value is `None`, the document has no `detail_duplicate` member.
///
/// The [`ProblemDetails`] from [`ProblemDetails::from`] has the string in `detail()` and in `extensions()`, under the
/// `detail_duplicate` name. [`ProblemDetails::with_detail`] on that value changes only the `detail` member.
///
/// `detail_duplicate` needs a detail source, as `detail_as` does. The macro rejects `detail_duplicate` together with
/// `detail_as`, because `detail_as` removes the `detail` member that `detail_duplicate` repeats.
///
/// ## `fixed(...)`
///
/// ```rust
/// use problem_details_fine::problem_type;
/// use serde_json::json;
///
/// #[problem_type(
///     type = "https://example.com/problems/widget-conflict",
///     status = CONFLICT,
///     title = "A widget with that id already exists",
///     fixed(
///         /// The legacy error code. It does not change between releases.
///         code: &str = "WIDGET_CONFLICT",
///         /// Always `false` for errors.
///         success: bool = false,
///         /// A member whose name is not a Rust identifier.
///         "error-kind": &str = "conflict",
///     ),
/// )]
/// #[derive(Debug)]
/// struct WidgetConflict;
///
/// assert_eq!(
///     serde_json::to_value(WidgetConflict).unwrap(),
///     json!({
///         "type": "https://example.com/problems/widget-conflict",
///         "status": 409,
///         "title": "A widget with that id already exists",
///         "code": "WIDGET_CONFLICT",
///         "success": false,
///         "error-kind": "conflict",
///     })
/// );
/// ```
///
/// Each entry is a doc comment, a name, a type and `= <literal>`:
///
/// - The doc comment is required; it becomes the `description` of the schema property. A missing doc comment is a
///   compile error.
/// - The name is a Rust identifier, such as `code`, or a string literal, such as `"error-kind"`. Use a string literal
///   for a name that is not a Rust identifier. The name of a raw identifier has no `r#`: `r#match` gives `match`. The
///   rules in [Collisions](#collisions) apply to both forms.
/// - The supported types are `&str` (or `&'static str`), `bool`, `i8`, `i16`, `i32`, `i64`, `isize`, `u8`, `u16`,
///   `u32`, `u64`, `usize`, `f32` and `f64`. The value must be a literal of the declared type.
///
/// Each fixed member is a required property of the Openapi schema. The schema of the property is
/// `enum: [<value>]`, with the value as its single allowed value.
///
/// Fixed members serialize at the top level of the document, in declaration order. They come after the `detail_as`
/// and `detail_duplicate` members and before the fields. With `wrap`, they are next to the `wrap` member, not in it.
/// [`ProblemDetails::from`] puts them in `extensions()`. That `serde_json::Map` sorts its keys, so the order there is
/// alphabetical.
///
/// Serde container attributes such as `rename_all` do not rename a fixed, `detail_as` or `detail_duplicate` member. The
/// document uses each name exactly as you write it. Nothing checks that fixed values, such as `code`, are unique
/// across problem types.
///
/// ## Collisions
///
/// The name of a `detail_as`, `detail_duplicate` or fixed member follows the same rules as a field. See
/// [Extension members](#extension-members) below. The name cannot be `type`, `status`, `title`, `detail` or
/// `instance`. It also cannot be the same as one of these names:
///
/// - A field, after serde renames.
/// - The `wrap` member.
/// - The `#[serde(tag = "...")]` or `#[serde(content = "...")]` member.
/// - Another fixed member, or the `detail_as` or `detail_duplicate` member.
/// - `tag`, if the `tag` argument adds the `tag` member. This rule is the same as for a field.
///
/// The compiler gives an error for each collision. It also gives an error for a fixed entry without a doc comment,
/// with an unsupported type, or with a value that does not match its declared type.
///
/// A `#[serde(flatten)]` field can add a member that the macro cannot see at compile time. If that member collides
/// with a `detail_as`, `detail_duplicate` or fixed member, serialization fails at run time. Serialization of the [`ProblemDetails`] from
/// [`ProblemDetails::from`] also fails, and the generation of the Openapi schema panics.
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
