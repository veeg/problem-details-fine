//! The global rewrite configuration: rules that change the type URIs and tag URIs of problems.
//!
//! A rewrite changes the serialized documents and the Openapi schemas in the same way. Build a [`RewriteConfig`]
//! from [`RewriteRule`]s, and install it with [`RewriteConfig::register_global`] at the start of `main`:
//!
//! ```rust
//! use problem_details_fine::{
//!     problem_type,
//!     rewrite::{RewriteConfig, RewriteRule, TagUri},
//! };
//!
//! #[problem_type(type = "tag:example.com,2025:WIDGET_CONFLICT", status = CONFLICT, title = "Conflict")]
//! #[derive(Debug)]
//! struct WidgetConflict;
//!
//! RewriteConfig::new()
//!     .rule(
//!         RewriteRule::exact("tag:example.com,2025:WIDGET_CONFLICT")
//!             .type_uri("https://docs.example.org/problems/widget-conflict")
//!             .tag_uri(TagUri::Keep),
//!     )
//!     .rule(RewriteRule::substring("example.com,2025", "example.org,2026"))
//!     .rule(RewriteRule::ignore("tag:third-party.example,"))
//!     .on_unmatched(|unmatched| eprintln!("no rewrite rule for {}", unmatched.type_uri()))
//!     .register_global()
//!     .unwrap();
//!
//! let document = serde_json::to_value(WidgetConflict).unwrap();
//! assert_eq!(document["type"], "https://docs.example.org/problems/widget-conflict");
//! assert_eq!(document["tag"], "tag:example.org,2026:WIDGET_CONFLICT");
//! ```
//!
//! The exact rule sets the type URI, and keeps the tag URI. The substring rule then changes the tagging entity of
//! the tag URI.
//!
//! Register the configuration once. The first read of the configuration fixes it, thus a later registration
//! fails. See [`RewriteConfig::register_global`].
//!
//! # Rules
//!
//! There are three kinds of rule. They apply in this order:
//!
//! 1. An exact rule, [`RewriteRule::exact`], matches one type URI as written in the definition. It can set a new
//!    type URI, and it can keep, replace or remove the tag URI.
//! 2. An ignore rule, [`RewriteRule::ignore`], matches text in the type URI or the tag URI. The problem then
//!    keeps its URIs as written.
//! 3. A substring rule, [`RewriteRule::substring`], replaces each occurrence of a text in the type URI and in
//!    the tag URI.
//!
//! If an exact rule matches, the ignore rules and the substring rules apply only to the URIs that the exact rule
//! keeps. A URI that the exact rule sets is final.
//!
//! For each URI, only one substring rule applies: the first that matches, in the order of the
//! [`RewriteConfig::rule`] calls. The order of the calls has no other effect.
//!
//! # The `tag` member
//!
//! A document has a `tag` member if the problem has a tag URI and the type URI is not a tag URI. Thus, an exact
//! rule that changes a tag URI `type` to an `https` URI adds a `tag` member, unless the rule removes the tag URI
//! with [`TagUri::Remove`].
//!
//! If the problem type also has an extension member `tag`, the document then has two `tag` members.
//! Serialization of the problem fails, and the generation of the Openapi documentation panics. Use
//! [`TagUri::Remove`] for such a problem type.
//!
//! A rewrite can also give two problem types in one problem set the same type URI. The generation of the
//! Openapi documentation then panics. See [`ProblemSet`](crate::ProblemSet#distinct-type-uris).
//!
//! # Problems without a rule
//!
//! [`RewriteConfig::on_unmatched`] registers a function that the crate calls for a problem that no rule
//! matches. Use it to find problem types that the configuration does not cover. See
//! [`RewriteConfig::on_unmatched`] for when the crate calls it.

use std::{borrow::Cow, collections::HashMap, fmt, sync::Arc, sync::OnceLock};

use crate::{identity::TypeIdentity, uri};

/// The function from [`RewriteConfig::on_unmatched`].
type OnUnmatched = Arc<dyn Fn(&Unmatched<'_>) + Send + Sync>;

/// The global rewrite configuration. Install it with [`RewriteConfig::register_global`].
#[derive(Clone, Default)]
pub struct RewriteConfig {
    exact: HashMap<Cow<'static, str>, ExactRule>,
    ignore: Vec<Cow<'static, str>>,
    substring: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    on_unmatched: Option<OnUnmatched>,
    /// The first error in the rules. [`RewriteConfig::register_global`] returns it.
    error: Option<RewriteError>,
}

/// One rule of a [`RewriteConfig`]. Make a rule with [`RewriteRule::exact`], [`RewriteRule::ignore`] or
/// [`RewriteRule::substring`].
#[derive(Debug, Clone)]
pub struct RewriteRule(Rule);

#[derive(Debug, Clone)]
enum Rule {
    Exact(ExactRule),
    Ignore(Cow<'static, str>),
    Substring(Cow<'static, str>, Cow<'static, str>),
}

/// A rule for one type URI. Make it with [`RewriteRule::exact`].
#[derive(Debug, Clone)]
pub struct ExactRule {
    from: Cow<'static, str>,
    type_uri: Option<Cow<'static, str>>,
    tag_uri: TagUri,
}

/// What an exact rule does with the tag URI of the problem type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TagUri {
    /// Keeps the tag URI. The ignore rules and the substring rules apply to it.
    #[default]
    Keep,
    /// Sets this tag URI. It must be a valid RFC 4151 tag URI. The other rules do not apply to it.
    Replace(Cow<'static, str>),
    /// Removes the tag URI. The document then has no `tag` member.
    Remove,
}

/// A problem that no rule matches. The function from [`RewriteConfig::on_unmatched`] gets it.
#[derive(Debug, Clone, Copy)]
pub struct Unmatched<'a> {
    type_uri: &'a str,
    tag_uri: Option<&'a str>,
}

/// An error from [`RewriteConfig::register_global`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RewriteError {
    /// Two exact rules have this type URI.
    DuplicateExact(String),
    /// Two substring rules replace this text.
    DuplicateSubstring(String),
    /// An ignore rule or a substring rule has an empty text.
    EmptyPattern,
    /// This value of [`TagUri::Replace`] is not a valid RFC 4151 tag URI.
    InvalidTagUri(String),
    /// A configuration is already registered, or the crate already read the configuration.
    AlreadyRegistered,
}

static CONFIG: OnceLock<RewriteConfig> = OnceLock::new();

/// Returns the global configuration. If no configuration is registered, this installs the empty configuration.
pub(crate) fn current() -> &'static RewriteConfig {
    CONFIG.get_or_init(RewriteConfig::default)
}

impl RewriteConfig {
    /// Makes an empty configuration. It does not change any URI.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a rule.
    pub fn rule(mut self, rule: impl Into<RewriteRule>) -> Self {
        match rule.into().0 {
            Rule::Exact(exact) => {
                if let TagUri::Replace(tag_uri) = &exact.tag_uri
                    && !uri::is_valid_tag(tag_uri)
                {
                    self.record(RewriteError::InvalidTagUri(tag_uri.to_string()));
                }
                if self.exact.contains_key(&exact.from) {
                    self.record(RewriteError::DuplicateExact(exact.from.to_string()));
                }
                self.exact.insert(exact.from.clone(), exact);
            }
            Rule::Ignore(pattern) => {
                if pattern.is_empty() {
                    self.record(RewriteError::EmptyPattern);
                }
                self.ignore.push(pattern);
            }
            Rule::Substring(from, to) => {
                if from.is_empty() {
                    self.record(RewriteError::EmptyPattern);
                }
                if self.substring.iter().any(|(existing, _)| *existing == from) {
                    self.record(RewriteError::DuplicateSubstring(from.to_string()));
                }
                self.substring.push((from, to));
            }
        }
        self
    }

    /// Registers a function that the crate calls for a problem that no rule matches.
    ///
    /// The function gets the type URI and the tag URI as written. The crate calls it when it applies the
    /// configuration to a problem:
    ///
    /// - For a problem type, the crate does this once in the life of the process: at the first serialization, or
    ///   when it generates the Openapi documentation.
    /// - For a [`ProblemDetails`](crate::ProblemDetails) without a source problem type, the crate does this each
    ///   time that you read or serialize the `type` or `tag` member.
    ///
    /// The function can panic, for example in a test that builds the Openapi documentation. Do not panic while
    /// the service handles requests.
    pub fn on_unmatched(
        mut self,
        on_unmatched: impl Fn(&Unmatched<'_>) + Send + Sync + 'static,
    ) -> Self {
        self.on_unmatched = Some(Arc::new(on_unmatched));
        self
    }

    /// Installs this configuration as the global configuration.
    ///
    /// Call this function at the start of `main`. The first read of the configuration fixes it. These operations
    /// read it:
    ///
    /// - the serialization of a problem;
    /// - [`ProblemDetails::type_uri`](crate::ProblemDetails::type_uri) and
    ///   [`ProblemDetails::tag_uri`](crate::ProblemDetails::tag_uri);
    /// - the generation of a schema.
    ///
    /// Returns the first error in the rules. Returns [`RewriteError::AlreadyRegistered`] after the first read, or
    /// after a first registration. Thus, the serialized documents and the Openapi documentation always use the
    /// same configuration.
    pub fn register_global(mut self) -> Result<(), RewriteError> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        CONFIG
            .set(self)
            .map_err(|_| RewriteError::AlreadyRegistered)
    }

    fn record(&mut self, error: RewriteError) {
        self.error.get_or_insert(error);
    }

    /// Applies the rules to the `type` and `tag` of a definition.
    ///
    /// `tag_uri` is the tag URI of the definition, [`ProblemType::TAG`](crate::ProblemType::TAG). If `type_uri` is
    /// a tag URI, `tag_uri` is equal to `type_uri`.
    pub(crate) fn resolve<'a>(
        &'a self,
        type_uri: &'a str,
        tag_uri: Option<&'a str>,
    ) -> TypeIdentity<'a> {
        let written = || TypeIdentity {
            type_uri: Cow::Borrowed(type_uri),
            tag_uri: tag_uri.map(Cow::Borrowed),
        };
        if self.exact.is_empty()
            && self.ignore.is_empty()
            && self.substring.is_empty()
            && self.on_unmatched.is_none()
        {
            return written();
        }

        if let Some(exact) = self.exact.get(type_uri) {
            let type_uri = match &exact.type_uri {
                Some(new) => Cow::Borrowed(new.as_ref()),
                None => self.general(type_uri).0,
            };
            let tag_uri = match &exact.tag_uri {
                TagUri::Keep => tag_uri.map(|tag_uri| self.general(tag_uri).0),
                TagUri::Replace(new) => Some(Cow::Borrowed(new.as_ref())),
                TagUri::Remove => None,
            };
            return TypeIdentity { type_uri, tag_uri };
        }

        if self.ignored(type_uri) || tag_uri.is_some_and(|tag_uri| self.ignored(tag_uri)) {
            return written();
        }

        let (new_type_uri, type_matched) = self.substitute(type_uri);
        let (new_tag_uri, tag_matched) = match tag_uri {
            Some(tag_uri) => {
                let (new, matched) = self.substitute(tag_uri);
                (Some(new), matched)
            }
            None => (None, false),
        };
        if !type_matched
            && !tag_matched
            && let Some(on_unmatched) = &self.on_unmatched
        {
            on_unmatched(&Unmatched { type_uri, tag_uri });
        }
        TypeIdentity {
            type_uri: new_type_uri,
            tag_uri: new_tag_uri,
        }
    }

    /// Applies the ignore rules and the substring rules to a URI that an exact rule keeps.
    fn general<'a>(&'a self, uri: &'a str) -> (Cow<'a, str>, bool) {
        if self.ignored(uri) {
            return (Cow::Borrowed(uri), true);
        }
        self.substitute(uri)
    }

    fn ignored(&self, uri: &str) -> bool {
        self.ignore
            .iter()
            .any(|pattern| uri.contains(pattern.as_ref()))
    }

    /// Applies the first substring rule that matches `uri`, to each occurrence of its text.
    fn substitute<'a>(&self, uri: &'a str) -> (Cow<'a, str>, bool) {
        match self
            .substring
            .iter()
            .find(|(from, _)| uri.contains(from.as_ref()))
        {
            Some((from, to)) => (Cow::Owned(uri.replace(from.as_ref(), to)), true),
            None => (Cow::Borrowed(uri), false),
        }
    }
}

impl fmt::Debug for RewriteConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RewriteConfig")
            .field("exact", &self.exact)
            .field("ignore", &self.ignore)
            .field("substring", &self.substring)
            .field("on_unmatched", &self.on_unmatched.is_some())
            .finish_non_exhaustive()
    }
}

impl RewriteRule {
    /// Makes a rule for the problem type with the type URI `type_uri`, as written in its definition.
    ///
    /// Without more calls, the rule keeps the type URI and the tag URI. The ignore rules and the substring rules
    /// then apply to them.
    pub fn exact(type_uri: impl Into<Cow<'static, str>>) -> ExactRule {
        ExactRule {
            from: type_uri.into(),
            type_uri: None,
            tag_uri: TagUri::Keep,
        }
    }

    /// Makes a rule that leaves a problem as written if its type URI or its tag URI contains `pattern`.
    ///
    /// The function from [`RewriteConfig::on_unmatched`] does not get this problem.
    pub fn ignore(pattern: impl Into<Cow<'static, str>>) -> Self {
        Self(Rule::Ignore(pattern.into()))
    }

    /// Makes a rule that replaces each occurrence of `from` with `to`, in the type URI and in the tag URI.
    ///
    /// For example, `RewriteRule::substring("example.com,2025", "example.org,2026")` changes
    /// `tag:example.com,2025:NOT_FOUND` to `tag:example.org,2026:NOT_FOUND`. Use a text that occurs only where you
    /// want the replacement.
    pub fn substring(from: impl Into<Cow<'static, str>>, to: impl Into<Cow<'static, str>>) -> Self {
        Self(Rule::Substring(from.into(), to.into()))
    }
}

impl ExactRule {
    /// Sets a new type URI. The other rules do not apply to it.
    pub fn type_uri(mut self, type_uri: impl Into<Cow<'static, str>>) -> Self {
        self.type_uri = Some(type_uri.into());
        self
    }

    /// Sets what the rule does with the tag URI. The default is [`TagUri::Keep`].
    pub fn tag_uri(mut self, tag_uri: TagUri) -> Self {
        self.tag_uri = tag_uri;
        self
    }
}

impl From<ExactRule> for RewriteRule {
    fn from(exact: ExactRule) -> Self {
        Self(Rule::Exact(exact))
    }
}

impl TagUri {
    /// Makes [`TagUri::Replace`].
    pub fn replace(tag_uri: impl Into<Cow<'static, str>>) -> Self {
        Self::Replace(tag_uri.into())
    }
}

impl<'a> Unmatched<'a> {
    /// Returns the type URI, as written.
    pub fn type_uri(&self) -> &'a str {
        self.type_uri
    }

    /// Returns the tag URI, as written.
    pub fn tag_uri(&self) -> Option<&'a str> {
        self.tag_uri
    }
}

impl fmt::Display for RewriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateExact(type_uri) => {
                write!(f, "two exact rewrite rules have the type URI `{type_uri}`")
            }
            Self::DuplicateSubstring(from) => {
                write!(f, "two substring rewrite rules replace `{from}`")
            }
            Self::EmptyPattern => f.write_str("a rewrite rule has an empty text"),
            Self::InvalidTagUri(tag_uri) => {
                write!(f, "`{tag_uri}` is not a valid RFC 4151 tag URI")
            }
            Self::AlreadyRegistered => f.write_str(
                "the rewrite configuration is already registered, or the crate already read it",
            ),
        }
    }
}

impl std::error::Error for RewriteError {}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    const TAG: &str = "tag:old.example,2025:NOT_FOUND";
    const HTTPS: &str = "https://old.example/problems/not-found";

    fn identity(type_uri: &str, tag_uri: Option<&str>) -> (String, Option<String>) {
        (type_uri.to_owned(), tag_uri.map(ToOwned::to_owned))
    }

    fn resolve(
        config: &RewriteConfig,
        type_uri: &str,
        tag_uri: Option<&str>,
    ) -> (String, Option<String>) {
        let resolved = config.resolve(type_uri, tag_uri);
        (
            resolved.type_uri.into_owned(),
            resolved.tag_uri.map(Cow::into_owned),
        )
    }

    #[test]
    fn empty_configuration_keeps_uris() {
        let config = RewriteConfig::new();
        assert_eq!(resolve(&config, TAG, Some(TAG)), identity(TAG, Some(TAG)));
        assert_eq!(resolve(&config, HTTPS, None), identity(HTTPS, None));
    }

    #[test]
    fn exact_rule_sets_type_and_controls_tag() {
        let rule = || RewriteRule::exact(TAG).type_uri("https://docs.test/not-found");
        let keep = RewriteConfig::new().rule(rule());
        assert_eq!(
            resolve(&keep, TAG, Some(TAG)),
            identity("https://docs.test/not-found", Some(TAG))
        );

        let replace =
            RewriteConfig::new().rule(rule().tag_uri(TagUri::replace("tag:new.example,2026:NF")));
        assert_eq!(
            resolve(&replace, TAG, Some(TAG)),
            identity(
                "https://docs.test/not-found",
                Some("tag:new.example,2026:NF")
            )
        );

        let remove = RewriteConfig::new().rule(rule().tag_uri(TagUri::Remove));
        assert_eq!(
            resolve(&remove, TAG, Some(TAG)),
            identity("https://docs.test/not-found", None)
        );
    }

    #[test]
    fn exact_rule_can_change_only_the_tag() {
        let config = RewriteConfig::new()
            .rule(RewriteRule::exact(HTTPS).tag_uri(TagUri::replace("tag:new.example,2026:NF")));
        assert_eq!(
            resolve(&config, HTTPS, None),
            identity(HTTPS, Some("tag:new.example,2026:NF"))
        );
    }

    #[test]
    fn kept_uris_go_through_general_rules_and_set_uris_do_not() {
        let config = RewriteConfig::new()
            .rule(RewriteRule::substring(
                "old.example,2025",
                "new.example,2026",
            ))
            .rule(RewriteRule::exact(TAG).type_uri("https://old.example,2025/x"));
        assert_eq!(
            resolve(&config, TAG, Some(TAG)),
            identity(
                "https://old.example,2025/x",
                Some("tag:new.example,2026:NOT_FOUND")
            )
        );

        let config = RewriteConfig::new()
            .rule(RewriteRule::substring(
                "old.example,2025",
                "new.example,2026",
            ))
            .rule(RewriteRule::exact(TAG));
        assert_eq!(
            resolve(&config, TAG, Some(TAG)),
            identity(
                "tag:new.example,2026:NOT_FOUND",
                Some("tag:new.example,2026:NOT_FOUND")
            )
        );
    }

    #[test]
    fn substring_rule_replaces_each_occurrence_and_first_match_wins() {
        let config = RewriteConfig::new()
            .rule(RewriteRule::substring("old", "new"))
            .rule(RewriteRule::substring("example", "sample"));
        assert_eq!(
            resolve(&config, "https://old.example/old", None),
            identity("https://new.example/new", None)
        );
        assert_eq!(
            resolve(&config, "https://example.test/x", None),
            identity("https://sample.test/x", None)
        );
    }

    #[test]
    fn substring_rule_applies_to_type_and_tag_separately() {
        let config = RewriteConfig::new().rule(RewriteRule::substring(
            "old.example,2025",
            "new.example,2026",
        ));
        assert_eq!(
            resolve(&config, HTTPS, Some(TAG)),
            identity(HTTPS, Some("tag:new.example,2026:NOT_FOUND"))
        );
    }

    #[test]
    fn ignore_rule_beats_substring_and_exact_beats_ignore() {
        let config = RewriteConfig::new()
            .rule(RewriteRule::substring("old", "new"))
            .rule(RewriteRule::ignore("old.example"));
        assert_eq!(resolve(&config, TAG, Some(TAG)), identity(TAG, Some(TAG)));

        let config = RewriteConfig::new()
            .rule(RewriteRule::ignore("old.example"))
            .rule(RewriteRule::exact(TAG).type_uri("https://docs.test/x"));
        assert_eq!(
            resolve(&config, TAG, Some(TAG)),
            identity("https://docs.test/x", Some(TAG))
        );
    }

    #[test]
    fn on_unmatched_gets_only_unmatched_problems() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = seen.clone();
        let config = RewriteConfig::new()
            .rule(RewriteRule::exact("https://exact.test/a"))
            .rule(RewriteRule::ignore("ignored.test"))
            .rule(RewriteRule::substring("old.example", "new.example"))
            .on_unmatched(move |unmatched| {
                record.lock().unwrap().push((
                    unmatched.type_uri().to_owned(),
                    unmatched.tag_uri().map(ToOwned::to_owned),
                ));
            });
        for (type_uri, tag_uri) in [
            ("https://exact.test/a", None),
            ("https://ignored.test/b", None),
            (HTTPS, None),
            ("https://other.test/c", Some("tag:other.test,2025:C")),
        ] {
            config.resolve(type_uri, tag_uri);
        }
        assert_eq!(
            *seen.lock().unwrap(),
            [(
                "https://other.test/c".to_owned(),
                Some("tag:other.test,2025:C".to_owned())
            )]
        );
    }

    #[test]
    fn rule_errors() {
        let error = |config: RewriteConfig| config.error.unwrap();
        assert_eq!(
            error(
                RewriteConfig::new()
                    .rule(RewriteRule::exact(TAG))
                    .rule(RewriteRule::exact(TAG))
            ),
            RewriteError::DuplicateExact(TAG.to_owned())
        );
        assert_eq!(
            error(
                RewriteConfig::new()
                    .rule(RewriteRule::substring("a", "b"))
                    .rule(RewriteRule::substring("a", "c"))
            ),
            RewriteError::DuplicateSubstring("a".to_owned())
        );
        assert_eq!(
            error(RewriteConfig::new().rule(RewriteRule::substring("", "b"))),
            RewriteError::EmptyPattern
        );
        assert_eq!(
            error(RewriteConfig::new().rule(RewriteRule::ignore(""))),
            RewriteError::EmptyPattern
        );
        assert_eq!(
            error(
                RewriteConfig::new()
                    .rule(RewriteRule::exact(TAG).tag_uri(TagUri::replace("tag:bad")))
            ),
            RewriteError::InvalidTagUri("tag:bad".to_owned())
        );
    }
}
