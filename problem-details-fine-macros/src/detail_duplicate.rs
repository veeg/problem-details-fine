//! `detail_duplicate` of `problem_type`: an extension member that repeats the rendered detail, with an optional doc
//! comment.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, LitStr, Token, meta::ParseNestedMeta};

use crate::fixed;

/// The `detail_duplicate` argument.
pub(crate) struct DetailDuplicate {
    /// The member name, exactly as written. The parser removes the `r#` of a raw identifier.
    pub name: String,
    pub name_span: Span,
    /// The doc comment. It is empty if the argument has no doc comment.
    pub doc: String,
}

/// Parses `detail_duplicate = "<member>"`, or `detail_duplicate(/// doc comment
/// <member>)`. In the parenthesized form, the member is an identifier or a string literal.
pub(crate) fn parse(meta: &ParseNestedMeta) -> syn::Result<DetailDuplicate> {
    if meta.input.peek(Token![=]) {
        let lit: LitStr = meta.value()?.parse()?;
        return Ok(DetailDuplicate {
            name: lit.value(),
            name_span: lit.span(),
            doc: String::new(),
        });
    }
    let content;
    syn::parenthesized!(content in meta.input);
    let doc = fixed::doc(
        &content.call(Attribute::parse_outer)?,
        "the `detail_duplicate` member",
    )?;
    let (name, name_span) = fixed::name(&content)?;
    if content.peek(Token![,]) {
        content.parse::<Token![,]>()?;
    }
    if !content.is_empty() {
        return Err(content.error("`detail_duplicate(..)` takes one member name"));
    }
    Ok(DetailDuplicate {
        name,
        name_span,
        doc,
    })
}

/// Makes the `__DETAIL_DUPLICATE` value.
pub(crate) fn expand(duplicate: Option<&DetailDuplicate>) -> TokenStream {
    let Some(DetailDuplicate { name, doc, .. }) = duplicate else {
        return quote!(::core::option::Option::None);
    };
    let description = if doc.is_empty() {
        quote!(::core::option::Option::None)
    } else {
        quote!(::core::option::Option::Some(#doc))
    };
    quote! {
        ::core::option::Option::Some(::problem_details_fine::__private::DetailDuplicate {
            name: #name,
            description: #description,
        })
    }
}
