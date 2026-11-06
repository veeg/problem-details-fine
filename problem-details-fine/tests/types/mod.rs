//! Problem types exercising every `problem_type` argument, shared by the integration tests.
#![allow(dead_code)]

use problem_details_fine::problem_type;
use serde::Serialize;

/// A unit problem type with a tag URI `type`.
#[problem_type(type = "tag:fixtures.test,2025:UNIT", status = NOT_FOUND, title = "Unit")]
#[derive(Debug, Clone)]
pub struct UnitProblem;

/// A struct with flattened members, renamed by serde.
#[problem_type(
    type = "https://fixtures.test/problems/flat",
    status = CONFLICT,
    title = "Flat",
    detail = "widget `{widget_id}` exists {count} times",
)]
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FlatProblem {
    /// The widget id.
    #[schema(example = "w-1")]
    pub widget_id: String,
    /// How often.
    pub count: u32,
    /// Only present sometimes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub note: Option<String>,
}

/// A struct nested under the `error` member.
#[problem_type(type = "https://fixtures.test/problems/wrapped", status = BAD_REQUEST, title = "Wrapped", wrap = "error")]
#[derive(Debug, Clone)]
pub struct WrappedProblem {
    /// The member named `type`, allowed because it is nested.
    #[serde(rename = "type")]
    pub kind: String,
}

/// An internally tagged enum, flattened.
#[problem_type(
    type = "tag:fixtures.test,2025:TAGGED",
    status = 422,
    title = "Tagged",
    detail = "something about the {thing}"
)]
#[derive(Debug, Clone)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaggedProblem {
    /// Without members.
    #[problem(detail = "plain")]
    Plain,
    /// With members.
    #[problem(detail = "field {name:?} is {value}")]
    Field { name: String, value: i64 },
    /// Uses the type-level detail.
    Fallback { thing: String },
}

/// An externally tagged enum, nested under `reason`.
#[problem_type(type = "tag:fixtures.test,2025:EXTERNAL", status = BAD_REQUEST, title = "External", wrap = "reason")]
#[derive(Debug, Clone)]
pub enum ExternalProblem {
    Simple,
    Complex { depth: u8 },
}

/// A tuple struct, nested under `value`.
#[problem_type(type = "tag:fixtures.test,2025:NEWTYPE", status = BAD_REQUEST, title = "Newtype", wrap = "value")]
#[derive(Debug, Clone)]
pub struct NewtypeProblem(pub u64);

/// A computed detail.
#[problem_type(
    type = "tag:fixtures.test,2025:COMPUTED",
    status = TOO_MANY_REQUESTS,
    title = "Computed",
    detail_with = ComputedProblem::describe,
)]
#[derive(Debug, Clone)]
pub struct ComputedProblem {
    pub retry_after: u32,
}

impl ComputedProblem {
    fn describe(&self) -> Option<String> {
        (self.retry_after > 0).then(|| format!("retry in {}s", self.retry_after))
    }
}

/// An `https` type with an explicit tag, and an overridden schema title.
#[problem_type(
    type = "https://fixtures.test/problems/explicit-tag",
    tag = "tag:fixtures.test,2025:EXPLICIT",
    status = SERVICE_UNAVAILABLE,
    title = "Explicit",
    schema_title = "ExplicitTitle",
)]
#[derive(Debug, Clone)]
pub struct ExplicitTagProblem;

/// A nested type with its own schema component.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

/// A struct referencing another schema, which must be inlined.
#[problem_type(type = "tag:fixtures.test,2025:REFERENCING", status = BAD_REQUEST, title = "Referencing")]
#[derive(Debug, Clone)]
pub struct ReferencingProblem {
    pub location: Location,
    pub locations: Vec<Location>,
}

/// A hand-written payload schema.
#[problem_type(
    type = "tag:fixtures.test,2025:MANUAL",
    status = BAD_REQUEST,
    title = "Manual",
    payload_schema = manual_payload_schema,
)]
#[derive(Debug, Clone)]
pub struct ManualProblem {
    pub opaque: serde_json::Value,
}

#[cfg(feature = "utoipa")]
fn manual_payload_schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
    use utoipa::openapi::{ObjectBuilder, schema::SchemaType};
    ObjectBuilder::new()
        .property(
            "opaque",
            ObjectBuilder::new().schema_type(SchemaType::AnyValue),
        )
        .required("opaque")
        .into()
}

#[cfg(not(feature = "utoipa"))]
fn manual_payload_schema() {}

/// A raw identifier field and a skipped field.
#[problem_type(
    type = "tag:fixtures.test,2025:RAW",
    status = BAD_REQUEST,
    title = "Raw",
    detail = "loop {loop}"
)]
#[derive(Debug, Clone)]
pub struct RawProblem {
    pub r#loop: String,
    #[serde(skip)]
    pub title: String,
}

/// An adjacently tagged enum: variant members are nested, so `title` is allowed inside.
#[problem_type(type = "tag:fixtures.test,2025:ADJACENT", status = BAD_REQUEST, title = "Adjacent")]
#[derive(Debug, Clone)]
#[serde(tag = "kind", content = "data")]
pub enum AdjacentProblem {
    Titled { title: String },
    Count(u32),
}

/// Variants and fields removed or renamed by configuration.
#[problem_type(
    type = "tag:fixtures.test,2025:CONFIGURED",
    status = BAD_REQUEST,
    title = "Configured",
    detail = "{x}"
)]
#[derive(Debug, Clone)]
#[serde(tag = "kind")]
pub enum ConfiguredProblem {
    Present {
        #[cfg_attr(all(), serde(rename = "renamed"))]
        x: u32,
    },
    #[cfg(any())]
    Absent { x: u32 },
}

/// A struct with its own `tag` member, and an `https` type URI without a tag URI.
#[problem_type(type = "https://fixtures.test/problems/own-tag", status = BAD_REQUEST, title = "Own tag")]
#[derive(Debug, Clone)]
pub struct OwnTagProblem {
    pub tag: String,
}

/// A struct with its own `tag` member, and a tag URI as type URI. A rewrite to an `https` URI adds a `tag` member.
#[problem_type(type = "tag:fixtures.test,2025:TAG_URI_OWN_TAG", status = BAD_REQUEST, title = "Tag URI own tag")]
#[derive(Debug, Clone)]
pub struct TagUriOwnTagProblem {
    pub tag: String,
}

/// A struct with a flattened field that adds a `tag` member.
#[problem_type(type = "tag:fixtures.test,2025:FLATTENED_TAG", status = BAD_REQUEST, title = "Flattened tag")]
#[derive(Debug, Clone)]
pub struct FlattenedTagProblem {
    #[serde(flatten)]
    pub labels: Labels,
}

/// Members for [`FlattenedTagProblem`].
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Labels {
    pub tag: String,
}

/// A transparent struct, nested under `value`, with a hand-written schema (utoipa ignores `transparent`).
#[problem_type(
    type = "tag:fixtures.test,2025:TRANSPARENT",
    status = BAD_REQUEST,
    title = "Transparent",
    wrap = "value",
    payload_schema = transparent_payload_schema
)]
#[derive(Debug, Clone)]
#[serde(transparent)]
pub struct TransparentProblem {
    pub inner: u32,
}

#[cfg(feature = "utoipa")]
fn transparent_payload_schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
    <u32 as utoipa::PartialSchema>::schema()
}

#[cfg(not(feature = "utoipa"))]
fn transparent_payload_schema() {}
