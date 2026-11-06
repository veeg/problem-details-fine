//! Validates type URIs and tag URIs when the macro expands.

/// A valid type URI or tag URI.
pub(crate) struct Uri {
    /// The specific part of a tag URI. `None` for a URI that is not a tag URI.
    pub(crate) tag_specific: Option<String>,
}

/// Validates an absolute URI for the `type` of a problem type.
pub(crate) fn parse_type(value: &str) -> Result<Uri, String> {
    let Some((scheme, rest)) = value.split_once(':') else {
        return Err(format!(
            "`{value}` is not an absolute URI: it has no scheme"
        ));
    };
    let mut chars = scheme.chars();
    let valid_scheme = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !valid_scheme {
        return Err(format!(
            "`{value}` is not an absolute URI: `{scheme}` is not a valid scheme"
        ));
    }
    if rest.is_empty() {
        return Err(format!(
            "`{value}` is not an absolute URI: it is empty after the scheme"
        ));
    }
    if let Some(c) = value.chars().find(|c| {
        c.is_whitespace()
            || c.is_control()
            || matches!(c, '"' | '<' | '>' | '\\' | '^' | '`' | '{' | '|' | '}')
    }) {
        return Err(format!("`{value}` is not a valid URI: it contains {c:?}"));
    }
    if value.eq_ignore_ascii_case("about:blank") {
        return Err("`about:blank` is reserved for problems without a type; use `ProblemDetails::new(status)`".into());
    }

    if scheme.eq_ignore_ascii_case("tag") {
        let specific = parse_tag(value)?;
        return Ok(Uri {
            tag_specific: Some(specific),
        });
    }

    Ok(Uri { tag_specific: None })
}

/// Validates an RFC 4151 tag URI, `tag:<authority>,<date>:<specific>`, and returns `<specific>`.
pub(crate) fn parse_tag(value: &str) -> Result<String, String> {
    let invalid = |reason: &str| {
        Err(format!(
            "`{value}` is not a valid tag URI ({reason}); expected `tag:<authority>,<date>:<specific>` (RFC 4151), e.g. `tag:example.com,2025:NOT_FOUND`"
        ))
    };
    let Some(rest) = value.strip_prefix("tag:") else {
        return invalid("it must start with `tag:`");
    };
    let Some((entity, specific)) = rest.split_once(':') else {
        return invalid("missing `:` before the specific part");
    };
    let Some((authority, date)) = entity.split_once(',') else {
        return invalid("missing `,` between authority and date");
    };
    let valid_authority = match authority.split_once('@') {
        Some((local, domain)) => !local.is_empty() && is_dns_name(domain),
        None => is_dns_name(authority),
    };
    if !valid_authority {
        return invalid("the authority must be a DNS name or an email address");
    }
    if !is_date(date) {
        return invalid("the date must be `YYYY`, `YYYY-MM` or `YYYY-MM-DD`");
    }
    if specific.is_empty() {
        return invalid("the specific part is empty");
    }
    Ok(specific.to_owned())
}

fn is_dns_name(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

fn is_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    let number = |part: &str, len: usize, range: std::ops::RangeInclusive<u32>| {
        part.len() == len
            && part.chars().all(|c| c.is_ascii_digit())
            && range.contains(&part.parse().unwrap_or(0))
    };
    match parts.as_slice() {
        [year] => number(year, 4, 0..=9999),
        [year, month] => number(year, 4, 0..=9999) && number(month, 2, 1..=12),
        [year, month, day] => {
            number(year, 4, 0..=9999) && number(month, 2, 1..=12) && number(day, 2, 1..=31)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_type_uris() {
        let uri = parse_type("https://example.com/problems/not-found?x=1").unwrap();
        assert!(uri.tag_specific.is_none());

        let uri = parse_type("tag:example.com,2025-01:NOT_FOUND").unwrap();
        assert_eq!(uri.tag_specific.as_deref(), Some("NOT_FOUND"));

        assert!(parse_type("tag:someone@example.com,2025-01-31:x").is_ok());
        assert!(parse_type("urn:example:not-found").is_ok());
    }

    #[test]
    fn rejects_invalid_uris() {
        for value in [
            "not-found",
            "/problems/not-found",
            "1http://x",
            "https:",
            "https://x/a b",
            "about:blank",
            "tag:example.com:2025:X",
            "tag:example.com,25:X",
            "tag:example.com,2025-13:X",
            "tag:-bad.com,2025:X",
            "tag:example.com,2025:",
        ] {
            assert!(parse_type(value).is_err(), "{value}");
        }
    }
}
