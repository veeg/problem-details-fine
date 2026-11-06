//! Reads the parts of `#[serde(..)]` attributes that set the names of the serialized members.

use syn::{
    Attribute, LitStr, Meta, Token, meta::ParseNestedMeta, parse_quote, punctuated::Punctuated,
};

#[derive(Default)]
pub(crate) struct SerdeContainer {
    pub(crate) rename_all: Option<LitStr>,
    pub(crate) rename_all_fields: Option<LitStr>,
    pub(crate) tag: Option<LitStr>,
    pub(crate) content: Option<LitStr>,
    /// `transparent`, `into` or `remote`. With these, the serialized value is not the fields of the item.
    pub(crate) opaque: Option<&'static str>,
}

#[derive(Default)]
pub(crate) struct SerdeMember {
    pub(crate) rename: Option<LitStr>,
    pub(crate) rename_all: Option<LitStr>,
    pub(crate) skip: bool,
    pub(crate) flatten: bool,
}

pub(crate) fn container(attrs: &[Attribute]) -> syn::Result<SerdeContainer> {
    let mut serde = SerdeContainer::default();
    for attr in serde_attrs(attrs)? {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                serde.rename_all = serialize_value(&meta)?;
            } else if meta.path.is_ident("rename_all_fields") {
                serde.rename_all_fields = serialize_value(&meta)?;
            } else if meta.path.is_ident("tag") {
                serde.tag = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("content") {
                serde.content = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("transparent") {
                serde.opaque = Some("transparent");
            } else if meta.path.is_ident("into") {
                serde.opaque = Some("into");
                skip(&meta)?;
            } else if meta.path.is_ident("remote") {
                serde.opaque = Some("remote");
                skip(&meta)?;
            } else {
                skip(&meta)?;
            }
            Ok(())
        })?;
    }
    Ok(serde)
}

/// Returns the serde attributes of a field or a variant.
pub(crate) fn member(attrs: &[Attribute]) -> syn::Result<SerdeMember> {
    let mut serde = SerdeMember::default();
    for attr in serde_attrs(attrs)? {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                serde.rename = serialize_value(&meta)?;
            } else if meta.path.is_ident("rename_all") {
                serde.rename_all = serialize_value(&meta)?;
            } else if meta.path.is_ident("skip") || meta.path.is_ident("skip_serializing") {
                serde.skip = true;
            } else if meta.path.is_ident("flatten") {
                serde.flatten = true;
            } else {
                skip(&meta)?;
            }
            Ok(())
        })?;
    }
    Ok(serde)
}

/// Returns the `#[serde(..)]` attributes, including those in `#[cfg_attr(<predicate>, serde(..))]`. This function
/// treats each predicate as true, which is the safe choice for the checks.
fn serde_attrs(attrs: &[Attribute]) -> syn::Result<Vec<Attribute>> {
    let mut serde = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("serde") {
            serde.push(attr.clone());
        } else if attr.path().is_ident("cfg_attr") {
            let metas = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            for meta in metas.into_iter().skip(1) {
                if meta.path().is_ident("serde") {
                    serde.push(parse_quote!(#[#meta]));
                }
            }
        }
    }
    Ok(serde)
}

/// Reads `key = "value"` or `key(serialize = "value", ..)`.
fn serialize_value(meta: &ParseNestedMeta) -> syn::Result<Option<LitStr>> {
    if meta.input.peek(Token![=]) {
        return Ok(Some(meta.value()?.parse()?));
    }
    let mut value = None;
    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("serialize") {
            value = Some(nested.value()?.parse()?);
        } else {
            skip(&nested)?;
        }
        Ok(())
    })?;
    Ok(value)
}

fn skip(meta: &ParseNestedMeta) -> syn::Result<()> {
    if meta.input.peek(Token![=]) {
        meta.value()?.parse::<syn::Expr>()?;
    } else if meta.input.peek(syn::token::Paren) {
        meta.input.parse::<proc_macro2::TokenTree>()?;
    }
    Ok(())
}

/// Applies a serde `rename_all` rule to a field name, as serde does. Serde treats a field name as snake case.
pub(crate) fn rename_field(rule: &LitStr, field: &str) -> syn::Result<String> {
    let pascal = || {
        let mut pascal = String::new();
        let mut capitalize = true;
        for c in field.chars() {
            if c == '_' {
                capitalize = true;
            } else if capitalize {
                pascal.push(c.to_ascii_uppercase());
                capitalize = false;
            } else {
                pascal.push(c);
            }
        }
        pascal
    };
    Ok(match rule.value().as_str() {
        "lowercase" | "snake_case" => field.to_owned(),
        "UPPERCASE" | "SCREAMING_SNAKE_CASE" => field.to_ascii_uppercase(),
        "PascalCase" => pascal(),
        "camelCase" => {
            let pascal = pascal();
            let mut chars = pascal.chars();
            chars
                .next()
                .map(|first| first.to_ascii_lowercase().to_string() + chars.as_str())
                .unwrap_or_default()
        }
        "kebab-case" => field.replace('_', "-"),
        "SCREAMING-KEBAB-CASE" => field.to_ascii_uppercase().replace('_', "-"),
        other => {
            return Err(syn::Error::new_spanned(
                rule,
                format!("unknown serde `rename_all` rule `{other}`"),
            ));
        }
    })
}

/// Removes each attribute with the name `name`, and returns them.
pub(crate) fn take(attrs: &mut Vec<Attribute>, name: &str) -> Vec<Attribute> {
    let (taken, kept) = std::mem::take(attrs)
        .into_iter()
        .partition(|attr| attr.path().is_ident(name));
    *attrs = kept;
    taken
}

/// Returns the `#[cfg(..)]` attributes. The macros copy them to the code that they generate for a field or a variant.
pub(crate) fn cfgs(attrs: &[Attribute]) -> Vec<&Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .collect()
}

/// Keeps only the attributes that the payload twin needs: `doc`, `cfg`, `cfg_attr`, `serde` and `schema`.
pub(crate) fn retain_schema_relevant(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| {
        ["doc", "serde", "schema", "cfg", "cfg_attr"]
            .iter()
            .any(|name| attr.path().is_ident(name))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renames_fields_like_serde() {
        let rename = |rule: &str, field: &str| {
            rename_field(&LitStr::new(rule, proc_macro2::Span::call_site()), field).unwrap()
        };
        assert_eq!(rename("camelCase", "widget_id"), "widgetId");
        assert_eq!(rename("PascalCase", "widget_id"), "WidgetId");
        assert_eq!(rename("snake_case", "detail_"), "detail_");
        assert_eq!(rename("lowercase", "Status"), "Status");
        assert_eq!(rename("UPPERCASE", "status"), "STATUS");
        assert_eq!(rename("kebab-case", "a_b"), "a-b");
        assert_eq!(rename("SCREAMING-KEBAB-CASE", "a_b"), "A-B");
    }
}
