use std::borrow::Cow;

use http::StatusCode;
use serde::{Serialize, Serializer, ser::Error as _};
use serde_json::{Map, Value};

use crate::{
    ProblemType,
    identity::{self, Resolved, TypeIdentity},
    problem::r#type::{Fixed, TagField},
};

/// The error from [`ProblemDetails::try_with_extension`].
#[derive(Debug)]
pub struct ExtensionError {
    key: String,
    kind: ExtensionErrorKind,
}

#[derive(Debug)]
enum ExtensionErrorKind {
    Reserved,
    Serialize(serde_json::Error),
}

impl ExtensionError {
    /// The name of the extension member that the call did not set.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns `true` if the name is a reserved member: `type`, `status`, `title`, `detail` or `instance`.
    /// Returns `false` if the value did not serialize.
    pub fn is_reserved(&self) -> bool {
        matches!(self.kind, ExtensionErrorKind::Reserved)
    }
}

impl std::fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ExtensionErrorKind::Reserved => {
                write!(f, "`{}` is a reserved problem details member", self.key)
            }
            ExtensionErrorKind::Serialize(error) => {
                write!(
                    f,
                    "extension member `{}` failed to serialize: {error}",
                    self.key
                )
            }
        }
    }
}

impl std::error::Error for ExtensionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ExtensionErrorKind::Reserved => None,
            ExtensionErrorKind::Serialize(error) => Some(error),
        }
    }
}

/// The members that RFC 9457 and this crate define. An extension member cannot use these names.
pub(crate) const RESERVED: &[&str] = &["type", "status", "title", "detail", "instance"];

/// An [RFC 9457] problem details document.
///
/// Build a document with the `with_*` methods. You can also convert a [`ProblemType`] or a
/// [`problem_set`](crate::problem_set) enum into a document, and then change it before you send it:
///
/// ```rust
/// use http::StatusCode;
/// use problem_details_fine::ProblemDetails;
///
/// let details = ProblemDetails::new(StatusCode::CONFLICT)
///     .with_type_uri("https://example.com/problems/out-of-stock")
///     .with_title("Out of stock")
///     .with_detail("only 3 left")
///     .with_instance("/orders/42")
///     .with_extension("available", 3);
///
/// assert_eq!(
///     serde_json::to_value(&details).unwrap(),
///     serde_json::json!({
///         "type": "https://example.com/problems/out-of-stock",
///         "status": 409,
///         "title": "Out of stock",
///         "detail": "only 3 left",
///         "instance": "/orders/42",
///         "available": 3,
///     })
/// );
/// ```
///
/// The rewrite configuration (feature `rewrite`) applies to `type` and `tag` when you read or serialize them.
///
/// A handler that returns a problem type serializes it in one pass. A conversion into `ProblemDetails`
/// serializes the extension members into a map first. Thus, convert only if you change the document.
///
/// [RFC 9457]: https://www.rfc-editor.org/rfc/rfc9457.html
#[derive(Debug, Clone)]
pub struct ProblemDetails {
    type_uri: Cow<'static, str>,
    tag_uri: Option<Cow<'static, str>>,
    status: StatusCode,
    title: Cow<'static, str>,
    detail: Option<String>,
    instance: Option<String>,
    extensions: Map<String, Value>,
    /// The error if the extension members of the source problem type did not serialize. Serialization of the
    /// document returns this error. Serialization of the problem type itself fails at the same point.
    error: Option<String>,
    /// The cached `type` and `tag` of the source problem type. `with_type_uri` and `with_tag_uri` remove it.
    resolved: Option<fn() -> &'static Resolved>,
}

impl PartialEq for ProblemDetails {
    fn eq(&self, other: &Self) -> bool {
        // `resolved` is a cache of the rewritten `type_` and `tag`. It does not change the document.
        self.type_uri == other.type_uri
            && self.tag_uri == other.tag_uri
            && self.status == other.status
            && self.title == other.title
            && self.detail == other.detail
            && self.instance == other.instance
            && self.extensions == other.extensions
            && self.error == other.error
    }
}

impl ProblemDetails {
    /// Makes a document with `"type": "about:blank"`. The `title` is the canonical reason phrase of the status.
    ///
    /// Give a 4xx or 5xx status. In a debug build, a different status panics. A release build does not check
    /// the status.
    pub fn new(status: StatusCode) -> Self {
        debug_assert!(
            status.is_client_error() || status.is_server_error(),
            "problem details describe 4xx/5xx responses, got {status}"
        );
        Self {
            type_uri: Cow::Borrowed("about:blank"),
            tag_uri: None,
            status,
            title: Cow::Borrowed(status.canonical_reason().unwrap_or("Unknown Status")),
            detail: None,
            instance: None,
            extensions: Map::new(),
            error: None,
            resolved: None,
        }
    }

    /// Sets the `type` member: the URI that identifies the problem type.
    pub fn with_type_uri(mut self, type_uri: impl Into<Cow<'static, str>>) -> Self {
        self.type_uri = type_uri.into();
        self.resolved = None;
        self
    }

    /// Sets the tag URI of the problem type. If the `type` member is not a tag URI, the document has a `tag`
    /// member with this tag URI.
    ///
    /// Do not also add an extension member `tag` with [`with_extension`](ProblemDetails::with_extension). The
    /// document then has two `tag` members, and serialization fails.
    pub fn with_tag_uri(mut self, tag_uri: impl Into<Cow<'static, str>>) -> Self {
        self.tag_uri = Some(tag_uri.into());
        self.resolved = None;
        self
    }

    /// Sets the `title` member: a short summary of the problem type, for a human reader.
    pub fn with_title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the `detail` member: an explanation of this occurrence of the problem.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Sets the `instance` member: a URI reference that identifies this occurrence of the problem.
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// Adds an extension member, or replaces it.
    ///
    /// The value is a `serde_json::Value`, thus this method cannot fail. Give a number, a string, a `bool`, or a
    /// value from `serde_json::json!(..)`. For a different value, call `serde_json::to_value` first, or use
    /// [`ProblemDetails::try_with_extension`].
    ///
    /// This method ignores a reserved name: `type`, `status`, `title`, `detail` or `instance`. In a debug
    /// build, it panics. [`ProblemDetails::try_with_extension`] returns an error for a reserved name.
    pub fn with_extension(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        let key = key.into();
        if RESERVED.contains(&key.as_str()) {
            debug_assert!(false, "`{key}` is a reserved problem details member");
            return self;
        }
        self.extensions.insert(key, value.into());
        self
    }

    /// Adds an extension member from a `Serialize` value, or replaces it.
    ///
    /// Returns an error if the name is reserved, or if the value does not serialize.
    ///
    /// ```rust
    /// # use http::StatusCode;
    /// # use problem_details_fine::ProblemDetails;
    /// #[derive(serde::Serialize)]
    /// struct Stock {
    ///     available: u32,
    /// }
    ///
    /// let details = ProblemDetails::new(StatusCode::CONFLICT).try_with_extension("stock", Stock { available: 3 })?;
    /// assert_eq!(details.extensions()["stock"]["available"], 3);
    /// # Ok::<(), problem_details_fine::ExtensionError>(())
    /// ```
    pub fn try_with_extension(
        mut self,
        key: impl Into<String>,
        value: impl Serialize,
    ) -> Result<Self, ExtensionError> {
        let key = key.into();
        if RESERVED.contains(&key.as_str()) {
            return Err(ExtensionError {
                key,
                kind: ExtensionErrorKind::Reserved,
            });
        }
        match serde_json::to_value(value) {
            Ok(value) => {
                self.extensions.insert(key, value);
                Ok(self)
            }
            Err(error) => Err(ExtensionError {
                key,
                kind: ExtensionErrorKind::Serialize(error),
            }),
        }
    }

    /// Returns the `type` member, after the rewrite.
    pub fn type_uri(&self) -> Cow<'_, str> {
        self.identity().type_uri
    }

    /// Returns the tag URI of the problem type, after the rewrite.
    ///
    /// The document has a tag URI before the rewrite in these cases:
    ///
    /// - The type URI is a tag URI.
    /// - The source [`ProblemType`] has a `tag` argument.
    /// - You called [`with_tag_uri`](ProblemDetails::with_tag_uri).
    ///
    /// A rewrite rule can then keep, replace or remove the tag URI. The serialized document has a `tag` member
    /// only if the `type` member is not a tag URI. Thus, this value can be `Some` while the document has no `tag`
    /// member.
    pub fn tag_uri(&self) -> Option<Cow<'_, str>> {
        self.identity().tag_uri
    }

    /// Returns the `title` member.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the `status` member.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the `detail` member.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Returns the `instance` member.
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }

    /// Returns the extension members.
    pub fn extensions(&self) -> &Map<String, Value> {
        &self.extensions
    }

    fn identity(&self) -> TypeIdentity<'_> {
        match self.resolved {
            Some(resolved) => resolved().0.borrowed(),
            None => identity::resolve(&self.type_uri, self.tag_uri.as_deref()),
        }
    }

    fn record_error(&mut self, error: String) {
        self.error.get_or_insert(error);
    }

    /// Inserts a `detail_as`, `detail_duplicate` or fixed member into `extensions`. If a member with that name is already there, it
    /// records an error instead.
    ///
    /// A `#[serde(flatten)]` field can add a member that the macro cannot see at compile time. Only this run-time
    /// check can find that collision.
    fn insert_retrofit_member(&mut self, definition: &str, name: &'static str, value: Value) {
        if self.extensions.contains_key(name) {
            self.record_error(retrofit_collision(definition, name));
            return;
        }
        self.extensions.insert(name.to_owned(), value);
    }
}

impl<T: ProblemType> From<T> for ProblemDetails {
    fn from(problem: T) -> Self {
        let (detail, detail_as, duplicate) = split_detail::<T>(problem.detail());
        let mut details = Self {
            type_uri: Cow::Borrowed(T::TYPE),
            tag_uri: T::TAG.map(Cow::Borrowed),
            status: T::STATUS,
            title: Cow::Borrowed(T::TITLE),
            detail,
            instance: None,
            extensions: Map::new(),
            error: None,
            resolved: Some(T::__resolved),
        };
        match serde_json::to_value(ExtensionMembers(&problem)) {
            Ok(Value::Object(extensions)) => details.extensions = extensions,
            Ok(other) => details.record_error(format!(
                "the extension members of `{}` serialized as {other}, not as a map",
                T::TYPE
            )),
            Err(error) => {
                details.record_error(format!("extension members of `{}`: {error}", T::TYPE))
            }
        }
        if let Some((name, detail)) = detail_as {
            details.insert_retrofit_member(T::__ID, name, Value::String(detail));
        }
        if let Some((name, detail)) = duplicate {
            details.insert_retrofit_member(T::__ID, name, Value::String(detail));
        }
        for fixed in T::__FIXED {
            details.insert_retrofit_member(T::__ID, fixed.name, fixed.value.to_json());
        }
        details
    }
}

impl Serialize for ProblemDetails {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(error) = &self.error {
            return Err(S::Error::custom(error));
        }
        let identity = self.identity();
        let tag_member = identity.tag_member();
        if tag_member.is_some() && self.extensions.contains_key("tag") {
            return Err(S::Error::custom(tag_collision(None)));
        }
        Document {
            type_uri: &identity.type_uri,
            tag_uri: tag_member,
            status: self.status.as_u16(),
            title: &self.title,
            detail: self.detail.as_deref(),
            instance: self.instance.as_deref(),
            retrofit: Retrofit {
                detail: None,
                duplicate: None,
                fixed: &[],
            },
            extensions: &self.extensions,
        }
        .serialize(serializer)
    }
}

/// Serializes a problem type as its complete document. The `Serialize` implementations from
/// [`problem_type`](crate::problem_type) call this function.
pub fn serialize_problem<T: ProblemType, S: Serializer>(
    problem: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let identity = &T::__resolved().0;
    let tag_member = identity.tag_member();
    if tag_member.is_some() && T::__TAG_FIELD == TagField::Yes {
        return Err(S::Error::custom(tag_collision(Some(T::__ID))));
    }
    let (detail, retrofit_detail, duplicate) = split_detail::<T>(problem.detail());

    // A `#[serde(flatten)]` field can add a `tag` member, or a member that collides with a `detail_as`,
    // `detail_duplicate` or fixed member. Only serialization shows these members. The code serializes the extension members once and uses the
    // result for both checks.
    let probe_tag = tag_member.is_some() && T::__TAG_FIELD == TagField::Unknown;
    let has_retrofit_members =
        retrofit_detail.is_some() || duplicate.is_some() || !T::__FIXED.is_empty();
    if T::__OPEN_MEMBERS
        && (probe_tag || has_retrofit_members)
        && let Ok(Value::Object(members)) = serde_json::to_value(ExtensionMembers(problem))
    {
        if probe_tag && members.contains_key("tag") {
            return Err(S::Error::custom(tag_collision(Some(T::__ID))));
        }
        let retrofit_names = retrofit_detail
            .iter()
            .chain(&duplicate)
            .map(|(name, _)| *name)
            .chain(T::__FIXED.iter().map(|fixed| fixed.name));
        for name in retrofit_names {
            if members.contains_key(name) {
                return Err(S::Error::custom(retrofit_collision(T::__ID, name)));
            }
        }
    }
    Document {
        type_uri: &identity.type_uri,
        tag_uri: tag_member,
        status: T::STATUS.as_u16(),
        title: T::TITLE,
        detail: detail.as_deref(),
        instance: None,
        retrofit: Retrofit {
            detail: retrofit_detail,
            duplicate,
            fixed: T::__FIXED,
        },
        extensions: ExtensionMembers(problem),
    }
    .serialize(serializer)
}

/// A rendered detail, split into the places where the document of `T` writes it: the `detail` member, the
/// `detail_as` member, and the `detail_duplicate` member. Each one is `None` if the document does not have it.
#[allow(clippy::type_complexity)]
fn split_detail<T: ProblemType>(
    rendered: Option<String>,
) -> (
    Option<String>,
    Option<(&'static str, String)>,
    Option<(&'static str, String)>,
) {
    let duplicate = T::__DETAIL_DUPLICATE
        .zip(rendered.clone())
        .map(|(duplicate, detail)| (duplicate.name, detail));
    match T::__DETAIL_AS {
        Some(name) => (None, rendered.map(|detail| (name, detail)), duplicate),
        None => (rendered, None, duplicate),
    }
}

/// The error message if a document has a `tag` member from its tag URI, and also an extension member `tag`.
pub(crate) fn tag_collision(definition: Option<&str>) -> String {
    let source = match definition {
        Some(definition) => format!("the problem type `{definition}`"),
        None => "the document".to_owned(),
    };
    format!(
        "{source} has a `tag` member from its tag URI, and also an extension member `tag`; remove the tag URI, \
         for example with `TagUri::Remove` in the rewrite configuration, or rename the extension member"
    )
}

/// The error message if a `detail_as`, `detail_duplicate` or fixed member collides with a member that a `#[serde(flatten)]` field
/// adds. The macro cannot see this collision at compile time.
pub(crate) fn retrofit_collision(definition: &str, name: &str) -> String {
    format!(
        "the problem type `{definition}` writes the member `{name}` from `detail_as`, `detail_duplicate` or \
         `fixed`, and a flattened field also writes `{name}`; rename one of them"
    )
}

#[derive(Serialize)]
struct Document<'a, E> {
    #[serde(rename = "type")]
    type_uri: &'a str,
    #[serde(rename = "tag", skip_serializing_if = "Option::is_none")]
    tag_uri: Option<&'a str>,
    status: u16,
    title: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<&'a str>,
    #[serde(flatten)]
    retrofit: Retrofit,
    #[serde(flatten)]
    extensions: E,
}

/// The `detail_as`, `detail_duplicate` and fixed members of a problem type: a map with the `detail_as` entry first,
/// then the `detail_duplicate` entry, then the fixed entries in declaration order.
struct Retrofit {
    /// The `detail_as` name and the rendered detail. It is `None` if the type has no `detail_as`, or if the rendered
    /// detail is `None`.
    detail: Option<(&'static str, String)>,
    /// The `detail_duplicate` name and the rendered detail. It is `None` if the type has no `detail_duplicate`, or
    /// if the rendered detail is `None`.
    duplicate: Option<(&'static str, String)>,
    /// The fixed members, in declaration order.
    fixed: &'static [Fixed],
}

impl Serialize for Retrofit {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let len = self.fixed.len()
            + usize::from(self.detail.is_some())
            + usize::from(self.duplicate.is_some());
        let mut map = serializer.serialize_map(Some(len))?;
        for (name, detail) in self.detail.iter().chain(&self.duplicate) {
            map.serialize_entry(name, detail)?;
        }
        for fixed in self.fixed {
            map.serialize_entry(fixed.name, &fixed.value)?;
        }
        map.end()
    }
}

struct ExtensionMembers<'a, T>(&'a T);

impl<T: ProblemType> Serialize for ExtensionMembers<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        T::__serialize_extensions(self.0, serializer)
    }
}

/// The `ProblemTypeImpl::__serialize_extensions` of a problem type without extension members.
pub fn serialize_no_extensions<S: Serializer>(serializer: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    serializer.serialize_map(Some(0))?.end()
}

/// The `ProblemTypeImpl::__serialize_extensions` of a problem type with `wrap`.
pub fn serialize_wrapped<S: Serializer, V: Serialize>(
    serializer: S,
    key: &'static str,
    value: &V,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(Some(1))?;
    map.serialize_entry(key, value)?;
    map.end()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn to_string<T: Serialize>(value: &T) -> String {
        serde_json::to_string(value).unwrap()
    }

    #[test]
    fn about_blank() {
        assert_eq!(
            to_string(&ProblemDetails::new(StatusCode::TOO_MANY_REQUESTS)),
            r#"{"type":"about:blank","status":429,"title":"Too Many Requests"}"#
        );
        let details = ProblemDetails::new(StatusCode::from_u16(499).unwrap());
        assert_eq!(details.title(), "Unknown Status");
        assert_eq!(details.type_uri(), "about:blank");
        assert_eq!(details.tag_uri(), None);
    }

    #[test]
    fn builder_members_in_order() {
        let details = ProblemDetails::new(StatusCode::CONFLICT)
            .with_type_uri("https://x.test/conflict")
            .with_tag_uri("tag:x.test,2025:CONFLICT")
            .with_title("Conflict")
            .with_detail("d")
            .with_instance("/i")
            .with_extension("b", 2)
            .with_extension("a", json!([1]));
        let expected = r#"{"type":"https://x.test/conflict","tag":"tag:x.test,2025:CONFLICT","status":409,"title":"Conflict","detail":"d","instance":"/i","a":[1],"b":2}"#;
        assert_eq!(to_string(&details), expected);
        assert_eq!(
            details.tag_uri().as_deref(),
            Some("tag:x.test,2025:CONFLICT")
        );
        assert_eq!(details.extensions()["a"], json!([1]));
    }

    #[test]
    fn extension_tag_collides_with_tag_member() {
        let details = ProblemDetails::new(StatusCode::CONFLICT)
            .with_type_uri("https://x.test/conflict")
            .with_tag_uri("tag:x.test,2025:CONFLICT")
            .with_extension("tag", "mine");
        let error = serde_json::to_string(&details).unwrap_err().to_string();
        assert!(error.contains("also an extension member `tag`"), "{error}");

        // A tag URI as type URI: the document has no `tag` member for the tag URI.
        let details = ProblemDetails::new(StatusCode::CONFLICT)
            .with_type_uri("tag:x.test,2025:CONFLICT")
            .with_extension("tag", "mine");
        assert_eq!(serde_json::to_value(&details).unwrap()["tag"], "mine");
    }

    #[test]
    fn extension_replaces_existing() {
        let details = ProblemDetails::new(StatusCode::CONFLICT)
            .with_extension("a", 1)
            .with_extension("a", 2);
        assert_eq!(details.extensions()["a"], json!(2));
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn reserved_extension_is_ignored() {
        let details = ProblemDetails::new(StatusCode::CONFLICT).with_extension("title", "x");
        assert_eq!(details.title(), "Conflict");
        assert!(details.extensions().is_empty());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "reserved")]
    fn reserved_extension_asserts_in_debug() {
        let _ = ProblemDetails::new(StatusCode::CONFLICT).with_extension("title", "x");
    }

    #[test]
    fn try_with_extension_reports_at_the_call() {
        let mut map = std::collections::BTreeMap::new();
        map.insert(vec![1u8], 1);
        let error = ProblemDetails::new(StatusCode::CONFLICT)
            .try_with_extension("map", map)
            .unwrap_err();
        assert_eq!(error.key(), "map");
        assert!(!error.is_reserved());
        assert!(
            error
                .to_string()
                .contains("extension member `map` failed to serialize")
        );
        assert!(std::error::Error::source(&error).is_some());

        let error = ProblemDetails::new(StatusCode::CONFLICT)
            .try_with_extension("title", "x")
            .unwrap_err();
        assert!(error.is_reserved());

        let details = ProblemDetails::new(StatusCode::CONFLICT)
            .try_with_extension("list", vec![1, 2])
            .unwrap();
        assert_eq!(details.extensions()["list"], json!([1, 2]));
    }
}
