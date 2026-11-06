//! Parses and expands `detail = "..."` format strings.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, LitStr};

/// The prefix for the binding of each placeholder. With the prefix, a field name that is a keyword, such as
/// `r#loop`, works.
const BINDING_PREFIX: &str = "__detail_";

/// A parsed format string: the names of its placeholders, and the format string with the bindings as names.
struct Format {
    names: Vec<String>,
    rewritten: String,
}

fn parse(format: &LitStr) -> syn::Result<Format> {
    let value = format.value();
    let mut names = Vec::new();
    let mut rewritten = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                rewritten.push_str("{{");
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                rewritten.push_str("}}");
            }
            '{' => {
                let mut name = String::new();
                let mut spec = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    if c == ':' || !spec.is_empty() {
                        spec.push(c);
                    } else {
                        name.push(c);
                    }
                }
                let name = name.trim().to_owned();
                if name.is_empty() || name.chars().all(|c| c.is_ascii_digit()) {
                    return Err(syn::Error::new_spanned(
                        format,
                        "`detail` placeholders must name a field, like `{field}`",
                    ));
                }
                if spec.contains('$') || spec.contains('*') {
                    return Err(syn::Error::new_spanned(
                        format,
                        "`detail` placeholders cannot take their width or precision from arguments",
                    ));
                }
                let name = name.strip_prefix("r#").unwrap_or(&name).to_owned();
                rewritten.push('{');
                rewritten.push_str(BINDING_PREFIX);
                rewritten.push_str(&name);
                rewritten.push_str(&spec);
                rewritten.push('}');
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            c => rewritten.push(c),
        }
    }
    Ok(Format { names, rewritten })
}

/// Makes the expression that gives `Some(String)` from `format`. The placeholders get the values of the fields.
///
/// Returns two token streams: the pattern for the fields, after `Self` or `Self::Variant`, and the expression.
pub(crate) fn render(format: &LitStr, fields: &Fields) -> syn::Result<(TokenStream, TokenStream)> {
    let parsed = parse(format)?;
    let known: Vec<(String, &syn::Ident)> = fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .map(|ident| {
            let name = ident.to_string();
            (name.strip_prefix("r#").unwrap_or(&name).to_owned(), ident)
        })
        .collect();

    let mut bindings = Vec::new();
    for name in &parsed.names {
        let Some((_, field)) = known.iter().find(|(known, _)| known == name) else {
            let message = if known.is_empty() {
                format!("`detail` refers to `{{{name}}}`, but there are no named fields")
            } else {
                format!(
                    "`detail` refers to `{{{name}}}`, which is not a field; available fields: {}",
                    known
                        .iter()
                        .map(|(known, _)| format!("`{known}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            return Err(syn::Error::new_spanned(format, message));
        };
        bindings.push((*field, format_ident!("{}{}", BINDING_PREFIX, name)));
    }

    let fields_bound = bindings
        .iter()
        .map(|(field, binding)| quote!(#field: #binding));
    let pattern = match fields {
        Fields::Named(_) => quote!({ #(#fields_bound,)* .. }),
        Fields::Unnamed(_) => quote!((..)),
        Fields::Unit => quote!(),
    };
    let rewritten = LitStr::new(&parsed.rewritten, format.span());
    let args = bindings
        .iter()
        .map(|(_, binding)| quote!(#binding = #binding));
    let expr = quote!(::core::option::Option::Some(
        ::std::format!(#rewritten, #(#args),*)
    ));
    Ok((pattern, expr))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(value: &str) -> LitStr {
        LitStr::new(value, proc_macro2::Span::call_site())
    }

    fn placeholders(format: &str) -> syn::Result<Vec<String>> {
        parse(&lit(format)).map(|format| format.names)
    }

    #[test]
    fn finds_placeholders() {
        assert_eq!(
            placeholders("a {x} b {y:?} {{literal}} {x}{z}").unwrap(),
            ["x", "y", "z"]
        );
        assert_eq!(placeholders("{r#loop}").unwrap(), ["loop"]);
        assert!(placeholders("{}").is_err());
        assert!(placeholders("{0}").is_err());
        assert!(placeholders("{x:>w$}").is_err());
        assert!(placeholders("no placeholders").unwrap().is_empty());
    }

    #[test]
    fn rewrites_placeholders_to_bindings() {
        let parsed = parse(&lit("{{a}} {x:?} {r#loop} }}")).unwrap();
        assert_eq!(parsed.rewritten, "{{a}} {__detail_x:?} {__detail_loop} }}");
    }
}
