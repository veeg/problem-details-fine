// Only some features use each of these functions.
#![cfg_attr(not(feature = "rewrite"), allow(dead_code))]

/// Returns `(authority_and_date, specific)` if `uri` is a tag URI (`tag:<authority>,<date>:<specific>`).
pub(crate) fn split_tag(uri: &str) -> Option<(&str, &str)> {
    let rest = uri.strip_prefix("tag:")?;
    let (entity, specific) = rest.split_once(':')?;
    entity.contains(',').then_some((entity, specific))
}

pub(crate) fn is_tag(uri: &str) -> bool {
    split_tag(uri).is_some()
}

/// Returns `true` if `uri` is a valid RFC 4151 tag URI, `tag:<authority>,<date>:<specific>`. The macros crate
/// has the same check, with error messages, for the definitions of problem types.
pub(crate) fn is_valid_tag(uri: &str) -> bool {
    let Some((entity, specific)) = split_tag(uri) else {
        return false;
    };
    let Some((authority, date)) = entity.split_once(',') else {
        return false;
    };
    let valid_authority = match authority.split_once('@') {
        Some((local, domain)) => !local.is_empty() && is_dns_name(domain),
        None => is_dns_name(authority),
    };
    valid_authority && is_date(date) && !specific.is_empty()
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
    let number = |part: &str, len: usize, range: std::ops::RangeInclusive<u32>| {
        part.len() == len
            && part.chars().all(|c| c.is_ascii_digit())
            && range.contains(&part.parse().unwrap_or(0))
    };
    match value.split('-').collect::<Vec<_>>().as_slice() {
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
    fn splits_tag_uris() {
        assert_eq!(
            split_tag("tag:example.com,2025:NOT_FOUND"),
            Some(("example.com,2025", "NOT_FOUND"))
        );
        assert_eq!(
            split_tag("tag:example.com,2025:a:b"),
            Some(("example.com,2025", "a:b"))
        );
        assert_eq!(split_tag("https://example.com/x"), None);
        assert_eq!(split_tag("tag:example.com:NOT_FOUND"), None);
        assert!(!is_tag("about:blank"));
    }

    #[test]
    fn validates_tag_uris() {
        assert!(is_valid_tag("tag:example.com,2025:NOT_FOUND"));
        assert!(is_valid_tag("tag:someone@example.com,2025-01-31:x"));
        for invalid in [
            "tag:example.com,25:X",
            "tag:-bad.com,2025:X",
            "tag:example.com,2025:",
            "tag:example.com,2025-13:X",
            "https://example.com/x",
        ] {
            assert!(!is_valid_tag(invalid), "{invalid}");
        }
    }
}
