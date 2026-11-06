use std::borrow::Cow;

use crate::uri;

/// The `type` and `tag` of a problem, after the rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeIdentity<'a> {
    pub(crate) type_uri: Cow<'a, str>,
    pub(crate) tag_uri: Option<Cow<'a, str>>,
}

impl TypeIdentity<'_> {
    /// Returns the value of the `tag` member: the tag URI, if the type URI is not a tag URI. If this is `None`, the
    /// document has no `tag` member.
    pub(crate) fn tag_member(&self) -> Option<&str> {
        match &self.tag_uri {
            Some(tag_uri) if !uri::is_tag(&self.type_uri) => Some(tag_uri),
            _ => None,
        }
    }

    /// Returns a copy that borrows from this identity.
    pub(crate) fn borrowed(&self) -> TypeIdentity<'_> {
        TypeIdentity {
            type_uri: Cow::Borrowed(&self.type_uri),
            tag_uri: self.tag_uri.as_deref().map(Cow::Borrowed),
        }
    }
}

/// The `type` and `tag` of a problem type, after the rewrite. The crate calculates them once for each problem
/// type, and keeps them until the process stops.
///
/// The rewrite configuration does not change after its first read. Thus, the result for the `TYPE` and `TAG` of a
/// problem type does not change. `#[problem_type]` keeps the result in a `static OnceLock` for each problem type.
/// After the first serialization, a read is one atomic load, without a lock or an allocation.
#[doc(hidden)]
#[derive(Debug)]
pub struct Resolved(pub(crate) TypeIdentity<'static>);

impl Resolved {
    pub fn new(type_uri: &'static str, tag_uri: Option<&'static str>) -> Self {
        Self(resolve(type_uri, tag_uri))
    }
}

/// Applies the global rewrite configuration to the `type` and `tag` of a definition.
#[cfg(feature = "rewrite")]
pub(crate) fn resolve<'a>(type_uri: &'a str, tag_uri: Option<&'a str>) -> TypeIdentity<'a> {
    crate::rewrite::current().resolve(type_uri, tag_uri)
}

/// Returns the `type` and `tag` of a definition as written. Without the `rewrite` feature, there are no rules.
#[cfg(not(feature = "rewrite"))]
pub(crate) fn resolve<'a>(type_uri: &'a str, tag_uri: Option<&'a str>) -> TypeIdentity<'a> {
    TypeIdentity {
        type_uri: Cow::Borrowed(type_uri),
        tag_uri: tag_uri.map(Cow::Borrowed),
    }
}
