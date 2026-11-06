use http::StatusCode;
use serde::Serialize;

use crate::{ProblemDetails, ProblemType};

/// The set of problem types that an operation can return.
///
/// A value of a problem set holds one problem. The value serializes as the complete problem details document of
/// that problem, and it converts into a [`ProblemDetails`].
///
/// Each [`ProblemType`] is a problem set with one problem type. Define a problem set with more problem types
/// with [`problem_set`](crate::problem_set).
///
/// # Generic code
///
/// Other traits, for example `axum::response::IntoResponse`, are implemented for each problem set, not for all
/// `T: ProblemSet`. Thus, add the bound that you need, for example `T: ProblemSet + IntoResponse`. Alternatively,
/// convert the value into a [`ProblemDetails`].
///
/// # Distinct type URIs
///
/// A type URI identifies one problem type. Thus, each problem type in a set must have a different type URI. The
/// same problem type can occur more than once. See [`problem_set`](crate::problem_set#distinct-type-uris).
///
/// The compiler finds two problem types with the same type URI in the definitions. A rewrite configuration
/// (feature `rewrite`) can also give two problem types the same type URI. The compiler cannot find this case.
/// The crate finds it when it generates the Openapi responses of the set, and then it panics. Write a test that
/// builds your Openapi document, so that the panic occurs in the test and not in the service.
///
/// # Implementing
///
/// The [`problem_set`](crate::problem_set) attribute macro implements this trait for an enum. This crate
/// implements it for each [`ProblemType`]: a problem type is a problem set with one problem type. There are no
/// other implementations.
///
/// The trait also has a hidden supertrait. It is not part of the public API, and it can change in any release.
/// Do not implement `ProblemSet` by hand.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is neither a problem type nor a problem set",
    label = "not a problem type or problem set",
    note = "define problem types with `#[problem_type(..)]` and problem sets with `#[problem_set]`"
)]
pub trait ProblemSet: Serialize + Into<ProblemDetails> + ProblemSetImpl {
    /// The status code of the problem that this value holds.
    fn status(&self) -> StatusCode;
}

/// The hidden items of a [`ProblemSet`]. They are in a supertrait, so that an implementation of `ProblemSet` by hand
/// fails with the message below, instead of a list of the missing hidden items. Each problem type implements this
/// trait, so the message also applies to a member of a problem set that is not a problem type.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is neither a problem type nor a problem set",
    label = "not a problem type or problem set",
    note = "define problem types with `#[problem_type(..)]` and problem sets with `#[problem_set]`",
    note = "do not implement `ProblemSet` by hand"
)]
pub trait ProblemSetImpl {
    /// The number of problem types in this set. A problem type that occurs on two paths counts two times.
    const __COUNT: usize;

    /// The type URI, as written, and the definition of each problem type in this set. The order is depth-first,
    /// in declaration order.
    const __TYPE_URIS: &'static [(&'static str, &'static str)];

    /// Adds the problem types of this set to `out`, depth-first, in declaration order.
    #[cfg(feature = "utoipa")]
    fn __entries(out: &mut Vec<crate::__private::ProblemEntry>);
}

#[cfg(not(feature = "utoipa"))]
impl<T: ProblemType> ProblemSet for T {
    fn status(&self) -> StatusCode {
        T::STATUS
    }
}

#[cfg(not(feature = "utoipa"))]
impl<T: ProblemType> ProblemSetImpl for T {
    const __COUNT: usize = 1;
    const __TYPE_URIS: &'static [(&'static str, &'static str)] = &[(T::TYPE, T::__ID)];
}

#[cfg(feature = "utoipa")]
impl<T: ProblemType + utoipa::PartialSchema> ProblemSet for T {
    fn status(&self) -> StatusCode {
        T::STATUS
    }
}

#[cfg(feature = "utoipa")]
impl<T: ProblemType + utoipa::PartialSchema> ProblemSetImpl for T {
    const __COUNT: usize = 1;
    const __TYPE_URIS: &'static [(&'static str, &'static str)] = &[(T::TYPE, T::__ID)];

    fn __entries(out: &mut Vec<crate::__private::ProblemEntry>) {
        out.push(crate::openapi::entry_of::<T>());
    }
}

/// Copies `source` into `out` from index `start`, and returns the next index. The generated `__TYPE_URIS` of a problem
/// set uses this function in `const` evaluation.
pub const fn copy_types<const N: usize>(
    out: &mut [(&'static str, &'static str); N],
    start: usize,
    source: &[(&'static str, &'static str)],
) -> usize {
    let mut i = 0;
    while i < source.len() {
        out[start + i] = source[i];
        i += 1;
    }
    start + source.len()
}

/// Stops compilation if a problem type in `first` and a different problem type in `second` have the same type URI.
/// The generated code calls this function for each pair of members of a problem set, with the span of the second
/// member. The `problem_set` of a member checks the problem types inside that member.
pub const fn assert_distinct_members(
    first: &[(&'static str, &'static str)],
    first_member: &str,
    second: &[(&'static str, &'static str)],
    second_member: &str,
) {
    let mut i = 0;
    while i < first.len() {
        let mut j = 0;
        while j < second.len() {
            let (uri, definition) = first[i];
            let (other_uri, other_definition) = second[j];
            if str_eq(uri, other_uri) && !str_eq(definition, other_definition) {
                shared_type_uri(
                    uri,
                    [
                        (definition, first_member),
                        (other_definition, second_member),
                    ],
                );
            }
            j += 1;
        }
        i += 1;
    }
}

/// The longest message of [`shared_type_uri`]. A longer message ends with `...`.
const MESSAGE_LEN: usize = 1024;

/// Panics with a message that names the type URI, the two problem types and the members of the set that hold them.
/// A `const` panic can print only one string, so the function writes the message into a buffer.
const fn shared_type_uri(uri: &str, types: [(&str, &str); 2]) -> ! {
    let mut message = [0u8; MESSAGE_LEN];
    let mut len = push(
        &mut message,
        0,
        "two different problem types share the type URI `",
    );
    len = push(&mut message, len, uri);
    len = push(&mut message, len, "`: ");
    let mut i = 0;
    while i < types.len() {
        let (definition, member) = types[i];
        if i > 0 {
            len = push(&mut message, len, " and ");
        }
        len = push(&mut message, len, "`");
        len = push(&mut message, len, definition);
        len = push(&mut message, len, "`");
        if !is_definition(definition, member) {
            len = push(&mut message, len, " in `");
            len = push(&mut message, len, member);
            len = push(&mut message, len, "`");
        }
        i += 1;
    }
    match core::str::from_utf8(message.split_at(len).0) {
        Ok(message) => panic!("{}", message),
        // The buffer ended inside a character.
        Err(_) => panic!("two different problem types share a type URI"),
    }
}

/// Writes `s` into `buf` from index `len`, and returns the new length. If `s` does not fit, the message ends with
/// `...`.
const fn push(buf: &mut [u8; MESSAGE_LEN], mut len: usize, s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if len == MESSAGE_LEN - 3 {
            buf[len] = b'.';
            buf[len + 1] = b'.';
            buf[len + 2] = b'.';
            return MESSAGE_LEN;
        }
        if len == MESSAGE_LEN {
            return len;
        }
        buf[len] = bytes[i];
        len += 1;
        i += 1;
    }
    len
}

/// Whether the member is the problem type itself: `member` is `definition` or the end of its path.
const fn is_definition(definition: &str, member: &str) -> bool {
    let (definition, member) = (definition.as_bytes(), member.as_bytes());
    if member.len() > definition.len() {
        return false;
    }
    let start = definition.len() - member.len();
    let (prefix, suffix) = definition.split_at(start);
    let after_separator =
        start == 0 || (start >= 2 && prefix[start - 2] == b':' && prefix[start - 1] == b':');
    after_separator && bytes_eq(suffix, member)
}

const fn str_eq(a: &str, b: &str) -> bool {
    bytes_eq(a.as_bytes(), b.as_bytes())
}

const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_types() {
        // The same problem type in both members is correct.
        assert_distinct_members(
            &[("a", "m::A"), ("b", "m::B")],
            "S",
            &[("c", "m::C"), ("a", "m::A")],
            "m::A",
        );
        let mut out = [("", ""); 3];
        let next = copy_types(&mut out, 0, &[("a", "A")]);
        let next = copy_types(&mut out, next, &[("b", "B"), ("c", "C")]);
        assert_eq!(next, 3);
        assert_eq!(out, [("a", "A"), ("b", "B"), ("c", "C")]);
    }

    #[test]
    #[should_panic(
        expected = "two different problem types share the type URI `a`: `m::A` in `S` and `m::B`"
    )]
    fn conflicting_types() {
        assert_distinct_members(&[("b", "m::C"), ("a", "m::A")], "S", &[("a", "m::B")], "B");
    }

    #[test]
    fn message_is_truncated() {
        let long = "x".repeat(2 * MESSAGE_LEN);
        let uri: &'static str = long.leak();
        let message =
            std::panic::catch_unwind(|| shared_type_uri(uri, [("m::A", "A"), ("m::B", "B")]))
                .unwrap_err()
                .downcast::<String>()
                .unwrap();
        assert_eq!(message.len(), MESSAGE_LEN);
        assert!(message.ends_with("..."));
    }

    #[test]
    fn definition_of_member() {
        assert!(is_definition("m::A", "A"));
        assert!(is_definition("m::A", "m::A"));
        assert!(is_definition("A", "A"));
        assert!(!is_definition("m::BA", "A"));
        assert!(!is_definition("m::A", "S"));
    }
}
