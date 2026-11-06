//! Problem types for common errors of an HTTP API.
//!
//! Each problem type in this module has a type URI as a tag URI with the tagging entity [`AUTHORITY`], for example
//! `tag:veeg.dev,2026:UNAUTHORIZED`. With the `rewrite` feature, you can change the type URIs:
//!
//! - A substring rule replaces the tagging entity with your own.
//! - An exact rule gives one problem type a URI to your own documentation.
//!
//! ```rust
//! # #[cfg(feature = "rewrite")] {
//! use problem_details_fine::{
//!     embedded,
//!     rewrite::{RewriteConfig, RewriteRule},
//! };
//!
//! RewriteConfig::new()
//!     .rule(RewriteRule::substring(embedded::AUTHORITY, "example.com,2026"))
//!     .rule(
//!         RewriteRule::exact("tag:veeg.dev,2026:UNAUTHORIZED")
//!             .type_uri("https://docs.example.com/problems/unauthorized"),
//!     )
//!     .register_global()
//!     .unwrap();
//! # }
//! ```
//!
//! # The `kind` member
//!
//! An enum in this module is one problem type with more than one cause. The extension member `kind` names the
//! cause, for example `"kind": "MISSING_AUTHORIZATION"` for [`UnauthorizedProblem::MissingAuthorization`]. Its
//! value is the variant name in `SCREAMING_SNAKE_CASE`. A client can read `type` and then `kind` to find the
//! exact cause. A struct in this module has one cause, and its document has no `kind` member.
//!
//! # Media types
//!
//! The documentation in this module says "media type". The names `UnsupportedMimeProblem`, `MissingMimeProblem`
//! and `ParseMimeProblem`, and their type URIs, say "MIME". The two terms mean the same thing here.

use crate::problem_type;

/// The tagging entity, `<authority>,<date>`, of the tag URIs in this module.
pub const AUTHORITY: &str = "veeg.dev,2026";

fn strings(values: &[impl ToString]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

/// The server cannot continue with the request because of an unexpected condition.
#[problem_type(
    type = "tag:veeg.dev,2026:INTERNAL_SERVER_ERROR",
    status = INTERNAL_SERVER_ERROR,
    title = "The server encountered an exceptional circumstance and was unable to proceed with processing the request",
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternalServerErrorProblem;

/// The request does not have correct authentication.
#[problem_type(
    type = "tag:veeg.dev,2026:UNAUTHORIZED",
    status = UNAUTHORIZED,
    title = "The request lacked appropriate or acceptable authentication",
    detail_with = UnauthorizedProblem::describe,
)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UnauthorizedProblem {
    /// The credentials are not valid, or they are revoked.
    #[schema(title = "CREDENTIALS")]
    Credentials,
    /// The request has no `Authorization` header.
    #[schema(title = "MISSING_AUTHORIZATION")]
    MissingAuthorization,
    /// The `Authorization` header has no credentials.
    #[schema(title = "MISSING_CREDENTIALS")]
    MissingCredentials,
    /// The server cannot read the `Authorization` header, because its format is not correct.
    #[schema(title = "MALFORMED_HEADER")]
    MalformedHeader,
    /// The request uses an authentication scheme that the endpoint no longer supports.
    #[schema(title = "DEPRECATED_SCHEME")]
    DeprecatedScheme {
        /// The scheme in the `Authorization` header.
        found: String,
        /// The schemes that the endpoint supports.
        supported: Vec<String>,
    },
    /// The request uses an authentication scheme that the endpoint does not support.
    #[schema(title = "UNSUPPORTED_SCHEME")]
    UnsupportedScheme {
        /// The scheme in the `Authorization` header.
        found: String,
        /// The schemes that the endpoint supports.
        supported: Vec<String>,
    },
    /// The OIDC token in the request is not valid.
    #[schema(title = "OIDC_VALIDATION_ERROR")]
    OidcValidationError {
        /// A message that describes the validation error.
        message: String,
    },
}

impl UnauthorizedProblem {
    /// Makes [`UnauthorizedProblem::DeprecatedScheme`].
    pub fn deprecated_scheme(found: impl ToString, supported: &[impl ToString]) -> Self {
        Self::DeprecatedScheme {
            found: found.to_string(),
            supported: strings(supported),
        }
    }

    /// Makes [`UnauthorizedProblem::UnsupportedScheme`].
    pub fn unsupported_scheme(found: impl ToString, supported: &[impl ToString]) -> Self {
        Self::UnsupportedScheme {
            found: found.to_string(),
            supported: strings(supported),
        }
    }

    /// Makes [`UnauthorizedProblem::OidcValidationError`].
    pub fn oidc_validation(message: impl ToString) -> Self {
        Self::OidcValidationError {
            message: message.to_string(),
        }
    }

    fn describe(&self) -> Option<String> {
        Some(match self {
            Self::Credentials => "the credentials are either invalid or revoked".to_owned(),
            Self::MissingAuthorization => "no `Authorization` header present".to_owned(),
            Self::MissingCredentials => "missing credentials in `Authorization` header".to_owned(),
            Self::MalformedHeader => {
                "the `Authorization` header is malformed and can't be processed".to_owned()
            }
            Self::DeprecatedScheme { found, supported } => format!(
                "scheme `{found}` is no longer supported; supported schemes are: {}",
                supported.join(", ")
            ),
            Self::UnsupportedScheme { found, supported } => format!(
                "scheme `{found}` is not supported; supported schemes are: {}",
                supported.join(", ")
            ),
            Self::OidcValidationError { message } => {
                format!("OIDC token validation failed: {message}")
            }
        })
    }
}

/// The credentials do not give access to the endpoint.
#[problem_type(
    type = "tag:veeg.dev,2026:FORBIDDEN",
    status = FORBIDDEN,
    title = "The provided credentials do not have access to the requested endpoint",
    detail_with = ForbiddenProblem::describe,
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForbiddenProblem {
    /// The scopes that the credentials do not have.
    pub missing_scopes: Vec<String>,
}

impl ForbiddenProblem {
    /// Makes a `ForbiddenProblem` for the scopes in `missing_scopes`.
    pub fn new(missing_scopes: &[impl ToString]) -> Self {
        Self {
            missing_scopes: strings(missing_scopes),
        }
    }

    fn describe(&self) -> Option<String> {
        (!self.missing_scopes.is_empty()).then(|| {
            format!(
                "the provided credentials are missing the following scope(s): {}",
                self.missing_scopes.join(", ")
            )
        })
    }
}

/// The endpoint does not support the media type of the request.
#[problem_type(
    type = "tag:veeg.dev,2026:UNSUPPORTED_MIME",
    status = UNSUPPORTED_MEDIA_TYPE,
    title = "The provided media type is unsupported by this endpoint",
    detail_with = UnsupportedMimeProblem::describe,
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedMimeProblem {
    /// The header that contains the media type.
    pub header: String,
    /// The media type in the header.
    pub found: String,
    /// The media types that the endpoint supports.
    pub supported: Vec<String>,
}

impl UnsupportedMimeProblem {
    /// Makes an `UnsupportedMimeProblem` for the media type `found` in `header`.
    pub fn new(header: impl ToString, found: impl ToString, supported: &[impl ToString]) -> Self {
        Self {
            header: header.to_string(),
            found: found.to_string(),
            supported: strings(supported),
        }
    }

    fn describe(&self) -> Option<String> {
        Some(format!(
            "the provided `{}` header media type `{}` is unsupported; supported media types are: {}",
            self.header,
            self.found,
            self.supported.join(", ")
        ))
    }
}

/// The request does not have a header with a media type.
#[problem_type(
    type = "tag:veeg.dev,2026:MISSING_MIME",
    status = UNSUPPORTED_MEDIA_TYPE,
    title = "The HTTP request did not provide a header containing a media type when expected",
    detail = "the expected `{header}` header is missing from the request",
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingMimeProblem {
    /// The header that the request does not have.
    pub header: String,
}

impl MissingMimeProblem {
    /// Makes a `MissingMimeProblem` for `header`.
    pub fn new(header: impl ToString) -> Self {
        Self {
            header: header.to_string(),
        }
    }
}

/// The server cannot parse a header that contains a media type.
#[problem_type(
    type = "tag:veeg.dev,2026:PARSE_MIME",
    status = BAD_REQUEST,
    title = "An HTTP header containing a media type encountered a parsing issue",
    detail_with = ParseMimeProblem::describe,
)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParseMimeProblem {
    /// The header value has characters that are not visible ASCII.
    #[schema(title = "INVALID_ASCII")]
    InvalidAscii {
        /// The header that contains the media type.
        header: String,
        /// The bytes of the header value.
        raw_value: Vec<u8>,
    },
    /// The header value is not a valid media type.
    #[schema(title = "PARSE_ERROR")]
    ParseError {
        /// The header that contains the media type.
        header: String,
        /// A message that describes the parse error.
        message: String,
        /// The bytes of the header value.
        raw_value: Vec<u8>,
    },
}

impl ParseMimeProblem {
    /// Makes [`ParseMimeProblem::InvalidAscii`].
    pub fn invalid_ascii(header: impl ToString, raw_value: Vec<u8>) -> Self {
        Self::InvalidAscii {
            header: header.to_string(),
            raw_value,
        }
    }

    /// Makes [`ParseMimeProblem::ParseError`].
    pub fn parse_error(header: impl ToString, message: impl ToString, raw_value: Vec<u8>) -> Self {
        Self::ParseError {
            header: header.to_string(),
            message: message.to_string(),
            raw_value,
        }
    }

    fn describe(&self) -> Option<String> {
        Some(match self {
            Self::InvalidAscii { header, raw_value } => format!(
                "the provided `{header}` header with value `{}` could not be parsed as only visible ASCII",
                String::from_utf8_lossy(raw_value)
            ),
            Self::ParseError {
                header,
                message,
                raw_value,
            } => format!(
                "the provided `{header}` header with value `{}` could not be parsed as a media type: {message}",
                String::from_utf8_lossy(raw_value)
            ),
        })
    }
}

/// The request does not have a body.
#[problem_type(
    type = "tag:veeg.dev,2026:MISSING_REQUEST_BODY",
    status = BAD_REQUEST,
    title = "The HTTP request did not provide a body when expected",
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingRequestBodyProblem;

/// The API has no operation on the path of the request.
#[problem_type(
    type = "tag:veeg.dev,2026:UNKNOWN_PATH_OPERATION",
    status = NOT_FOUND,
    title = "The HTTP path is unrecognized; there is no operation available on this path",
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownPathOperationProblem;

/// The server cannot parse a path parameter.
#[problem_type(
    type = "tag:veeg.dev,2026:PARSE_PATH_PARAMETER",
    status = BAD_REQUEST,
    title = "A dynamic HTTP path parameter encountered a parsing issue",
)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParsePathParameterProblem {
    /// The path parameter does not have the expected type.
    #[schema(title = "PARSE_ERROR")]
    #[problem(
        detail = "failed to parse path parameter `{value}` into expected type `{expected_type}`"
    )]
    ParseError {
        /// The value of the path parameter.
        value: String,
        /// The expected type of the path parameter.
        expected_type: String,
    },
    /// The path parameter does not deserialize.
    #[schema(title = "DESERIALIZE")]
    #[problem(detail = "failed to deserialize path parameter `{key} = {value}`: {message}")]
    Deserialize {
        /// The name of the path parameter.
        key: String,
        /// The value of the path parameter.
        value: String,
        /// A message that describes the deserialization error.
        message: String,
    },
}

impl ParsePathParameterProblem {
    /// Makes [`ParsePathParameterProblem::Deserialize`].
    pub fn deserialize(key: impl ToString, value: impl ToString, message: impl ToString) -> Self {
        Self::Deserialize {
            key: key.to_string(),
            value: value.to_string(),
            message: message.to_string(),
        }
    }

    /// Makes [`ParsePathParameterProblem::ParseError`].
    pub fn parse_error(value: impl ToString, expected_type: impl ToString) -> Self {
        Self::ParseError {
            value: value.to_string(),
            expected_type: expected_type.to_string(),
        }
    }
}

/// The server cannot parse the query string.
#[problem_type(
    type = "tag:veeg.dev,2026:PARSE_QUERY_PARAMETER",
    status = BAD_REQUEST,
    title = "An HTTP query parameter encountered a parsing issue",
)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParseQueryParameterProblem {
    /// The query string does not have a required parameter.
    #[schema(title = "MISSING_REQUIRED_FIELD")]
    #[problem(detail = "missing required query parameter `{field}`")]
    MissingRequiredField {
        /// The name of the parameter.
        field: String,
    },
    /// A query parameter does not deserialize into the expected type.
    #[schema(title = "DESERIALIZE")]
    #[problem(
        detail = "parsing URL query encountered a deserialization issue on field `{field}`: {message}"
    )]
    Deserialize {
        /// The name of the parameter.
        field: String,
        /// A message that describes the deserialization error.
        message: String,
        /// The value of the parameter, if it is known.
        value: Option<String>,
        /// The expected type of the parameter, if it is known.
        expected_type: Option<String>,
    },
}

impl ParseQueryParameterProblem {
    /// Makes [`ParseQueryParameterProblem::Deserialize`].
    pub fn deserialize(
        field: impl ToString,
        message: impl ToString,
        value: Option<impl ToString>,
        expected_type: Option<impl ToString>,
    ) -> Self {
        Self::Deserialize {
            field: field.to_string(),
            message: message.to_string(),
            value: value.map(|v| v.to_string()),
            expected_type: expected_type.map(|t| t.to_string()),
        }
    }

    /// Makes [`ParseQueryParameterProblem::MissingRequiredField`].
    pub fn missing_required_field(field: impl ToString) -> Self {
        Self::MissingRequiredField {
            field: field.to_string(),
        }
    }
}

/// The types of a JSON value, in the `TypeMismatch` variant of `ParseRequestBodyProblem`.
///
/// JSON has one number type. This enum has two, `Float` and `Integer`, because a Rust type can accept an integer
/// and reject a number with a fraction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JsonType {
    /// A string.
    String,
    /// `true` or `false`.
    Bool,
    /// A number with a fraction or an exponent.
    Float,
    /// A number without a fraction.
    Integer,
    /// An array.
    Array,
    /// An object.
    Object,
    /// `null`.
    Null,
}

/// The server cannot parse the request body as JSON of the expected type.
#[problem_type(
    type = "tag:veeg.dev,2026:PARSE_REQUEST_BODY",
    status = BAD_REQUEST,
    title = "The HTTP request body encountered a parsing issue",
    detail_with = ParseRequestBodyProblem::describe,
)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParseRequestBodyProblem {
    /// The JSON does not have a field that the expected type requires.
    #[schema(title = "MISSING_REQUIRED_FIELD")]
    MissingRequiredField {
        /// The name of the field.
        field: String,
    },
    /// The JSON does not deserialize into the expected type.
    #[schema(title = "DESERIALIZE")]
    Deserialize {
        /// The path to the field. The path does not follow a standard such as `JSONPath`.
        field: String,
        /// A message that describes the error.
        message: String,
        /// The value of the field, if it is known.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schema(nullable = false)]
        value: Option<String>,
        /// The line of the error in the request body.
        line: usize,
        /// The column of the error in the request body.
        column: usize,
    },
    /// A JSON value does not have the expected type.
    #[schema(title = "TYPE_MISMATCH")]
    TypeMismatch {
        /// The path to the field. The path does not follow a standard such as `JSONPath`.
        field: String,
        /// A message that describes the error.
        message: String,
        /// The value of the field, if it is known.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schema(nullable = false)]
        value: Option<String>,
        /// The type of the value in the request body, if it is known.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schema(nullable = false)]
        found_type: Option<JsonType>,
        /// The expected type, if it is known.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schema(nullable = false)]
        expected_type: Option<JsonType>,
        /// The line of the error in the request body.
        line: usize,
        /// The column of the error in the request body.
        column: usize,
    },
    /// The request body is not valid JSON.
    #[schema(title = "SYNTAX")]
    Syntax {
        /// The path to the field with the error. The path does not follow a standard such as `JSONPath`.
        field: String,
        /// The message from the JSON parser.
        message: String,
        /// The line of the error in the request body.
        line: usize,
        /// The column of the error in the request body.
        column: usize,
    },
}

impl ParseRequestBodyProblem {
    /// Makes [`ParseRequestBodyProblem::MissingRequiredField`].
    pub fn missing_required_field(field: impl ToString) -> Self {
        Self::MissingRequiredField {
            field: field.to_string(),
        }
    }

    /// Makes [`ParseRequestBodyProblem::Deserialize`].
    pub fn deserialize(
        field: impl ToString,
        message: impl ToString,
        value: Option<impl ToString>,
        line: usize,
        column: usize,
    ) -> Self {
        Self::Deserialize {
            field: field.to_string(),
            message: message.to_string(),
            value: value.map(|v| v.to_string()),
            line,
            column,
        }
    }

    /// Makes [`ParseRequestBodyProblem::TypeMismatch`].
    #[allow(clippy::too_many_arguments)]
    pub fn type_mismatch(
        field: impl ToString,
        message: impl ToString,
        value: Option<impl ToString>,
        found_type: Option<JsonType>,
        expected_type: Option<JsonType>,
        line: usize,
        column: usize,
    ) -> Self {
        Self::TypeMismatch {
            field: field.to_string(),
            message: message.to_string(),
            value: value.map(|v| v.to_string()),
            found_type,
            expected_type,
            line,
            column,
        }
    }

    /// Makes [`ParseRequestBodyProblem::Syntax`].
    pub fn syntax(
        field: impl ToString,
        message: impl ToString,
        line: usize,
        column: usize,
    ) -> Self {
        Self::Syntax {
            field: field.to_string(),
            message: message.to_string(),
            line,
            column,
        }
    }

    fn describe(&self) -> Option<String> {
        Some(match self {
            Self::MissingRequiredField { field } => {
                format!("missing required field `{field}` in request body")
            }
            Self::TypeMismatch {
                field,
                message,
                value,
                found_type,
                expected_type,
                ..
            } => {
                let mut detail = format!("type mismatch in request body field `{field}`; ");
                if let Some(value) = value {
                    detail.push_str(&format!("found value `{value}` "));
                    match found_type {
                        Some(found_type) => detail.push_str(&format!("of type `{found_type:?}`; ")),
                        None => detail.push_str("; "),
                    }
                }
                if let Some(expected_type) = expected_type {
                    detail.push_str(&format!("expected type `{expected_type:?}`; "));
                }
                detail.push_str(&format!("message: `{message}`"));
                detail
            }
            Self::Deserialize {
                field,
                message,
                value,
                ..
            } => {
                let mut detail = format!("failed to deserialize request body field `{field}`");
                match value {
                    Some(value) => detail.push_str(&format!(" with value `{value}`; ")),
                    None => detail.push_str("; "),
                }
                detail.push_str(&format!("message: {message}"));
                detail
            }
            Self::Syntax { field, message, .. } => {
                format!("syntax error in request body field `{field}`: {message}")
            }
        })
    }
}

/// An unexpected error occurred when the server processed the request body.
#[problem_type(
    type = "tag:veeg.dev,2026:UNHANDLED_REQUEST_BODY_ERROR",
    status = INTERNAL_SERVER_ERROR,
    title = "Parsing and processing the HTTP request body encountered a fatal error",
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnhandledRequestBodyErrorProblem;

/// An unexpected error occurred when the server processed a path parameter.
#[problem_type(
    type = "tag:veeg.dev,2026:UNHANDLED_PATH_PARAMETER_ERROR",
    status = INTERNAL_SERVER_ERROR,
    title = "Parsing and processing the HTTP path parameter encountered a fatal error",
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnhandledPathParameterErrorProblem;

/// An unexpected error occurred when the server processed the query string.
#[problem_type(
    type = "tag:veeg.dev,2026:UNHANDLED_QUERY_STRING_ERROR",
    status = INTERNAL_SERVER_ERROR,
    title = "Parsing and processing the HTTP query string encountered a fatal error",
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnhandledQueryStringErrorProblem;

/// A downstream dependency did not respond, or it returned a response that the server cannot read.
#[problem_type(
    type = "tag:veeg.dev,2026:DEPENDENCY",
    status = SERVICE_UNAVAILABLE,
    title = "A downstream dependency was unresponsive or otherwise incomprehensible",
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependencyProblem;

/// The server cannot find a resource that the request refers to.
#[problem_type(
    type = "tag:veeg.dev,2026:UNKNOWN_RESOURCE",
    status = NOT_FOUND,
    title = "A resource that was expected to exist could not be found",
    detail = "the requested resource of type `{resource_type}` with identifier `{resource_identifier}` was not found",
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownResourceProblem {
    /// The type of the resource.
    pub resource_type: String,
    /// The identifier of the resource.
    pub resource_identifier: String,
}

impl UnknownResourceProblem {
    /// Makes an `UnknownResourceProblem` for the resource of type `resource_type` with `resource_identifier`.
    pub fn new(resource_type: impl ToString, resource_identifier: impl ToString) -> Self {
        Self {
            resource_type: resource_type.to_string(),
            resource_identifier: resource_identifier.to_string(),
        }
    }
}
