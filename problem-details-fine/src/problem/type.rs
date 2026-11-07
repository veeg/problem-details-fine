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

/// The constant value of a [`Fixed`] member, from `fixed(...)` of `problem_type`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FixedValue {
    Str(&'static str),
    Bool(bool),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
}

impl FixedValue {
    /// Returns the JSON value. It is equal to the value that `Serialize` writes. An `F32` value converts to `f64`
    /// through the `Display` text of the `f32`, so `0.1f32` becomes `0.1`, not `0.10000000149011612`.
    pub fn to_json(&self) -> serde_json::Value {
        match *self {
            FixedValue::Str(value) => serde_json::Value::from(value),
            FixedValue::Bool(value) => serde_json::Value::from(value),
            FixedValue::I64(value) => serde_json::Value::from(value),
            FixedValue::U64(value) => serde_json::Value::from(value),
            FixedValue::F32(value) => serde_json::Value::from(f32_as_f64(value)),
            FixedValue::F64(value) => serde_json::Value::from(value),
        }
    }
}

impl Serialize for FixedValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            FixedValue::Str(value) => serializer.serialize_str(value),
            FixedValue::Bool(value) => serializer.serialize_bool(value),
            FixedValue::I64(value) => serializer.serialize_i64(value),
            FixedValue::U64(value) => serializer.serialize_u64(value),
            // Not `serialize_f32`: `serde_json::to_value` converts an `f32` to the `f64` with the same value, so
            // `0.1f32` becomes `0.10000000149011612`. `serde_json::to_string` writes `0.1`. The `f64` from the
            // `Display` text gives `0.1` in both, and the document agrees with the schema.
            FixedValue::F32(value) => serializer.serialize_f64(f32_as_f64(value)),
            FixedValue::F64(value) => serializer.serialize_f64(value),
        }
    }
}

/// Converts an `f32` to the `f64` with the same decimal text, instead of the `f64` with the same value.
fn f32_as_f64(value: f32) -> f64 {
    value
        .to_string()
        .parse()
        .expect("a f32's `Display` output parses as f64")
}

/// One `fixed(...)` member of a problem type: a constant extension member with a declared Rust type and
/// documentation.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fixed {
    /// The member name, written exactly as given.
    pub name: &'static str,
    /// The doc comment, used as the schema description.
    pub description: &'static str,
    /// The constant value.
    pub value: FixedValue,
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
    ///
    /// With `detail_as`, the document has this value in the `detail_as` member instead of `detail`. This method
    /// still returns the value. A caller can use it to log the problem.
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

    /// The name of the extension member that holds the rendered detail, from `detail_as`. It is `None` if the
    /// document has a standard `detail` member instead.
    const __DETAIL_AS: Option<&'static str>;

    /// Whether the schema marks the `__DETAIL_AS` member required.
    const __DETAIL_AS_REQUIRED: bool;

    /// The `fixed(...)` members, in declaration order.
    const __FIXED: &'static [Fixed];

    /// Whether a `#[serde(flatten)]` field can add top-level members the macro cannot see.
    const __OPEN_MEMBERS: bool;

    /// The path of the definition, `module::path::Ident`. It identifies the problem type in `const` evaluation.
    const __ID: &'static str;

    /// Serializes the extension members as a map, as `__EXTENSIONS` specifies.
    fn __serialize_extensions<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;

    /// The `type` and `tag` after the rewrite. The crate calculates them once for each problem type.
    fn __resolved() -> &'static crate::__private::Resolved;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn fixed_f32_is_exact_on_every_path() {
        let value = FixedValue::F32(0.1);
        assert_eq!(serde_json::to_string(&value).unwrap(), "0.1");
        assert_eq!(value.to_json(), json!(0.1));
        assert_eq!(serde_json::to_value(value).unwrap(), json!(0.1));
    }

    #[test]
    fn fixed_value_to_json_matches_serialize() {
        for value in [
            FixedValue::Str("A"),
            FixedValue::Bool(false),
            FixedValue::I64(-3),
            FixedValue::U64(u64::MAX),
            FixedValue::F64(1.25),
        ] {
            assert_eq!(serde_json::to_value(value).unwrap(), value.to_json());
        }
    }
}
