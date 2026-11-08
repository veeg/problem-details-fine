//! `fixed(...)` of `problem_type`: extension members with a constant value, a declared Rust type and a doc comment.

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    Attribute, Expr, ExprLit, Ident, Lit, LitStr, Token, Type, UnOp, parse::ParseStream,
    spanned::Spanned,
};

/// One entry of `fixed(...)`.
pub(crate) struct FixedEntry {
    /// The member name, exactly as written. The parser removes the `r#` of a raw identifier.
    pub name: String,
    pub name_span: Span,
    /// The doc comment. It is empty if the entry has no doc comment.
    pub doc: String,
    pub ty: Type,
    pub value: Expr,
}

/// Parses the parenthesized content of `fixed(...)`: entries separated by commas, with an optional trailing comma.
pub(crate) fn parse(input: ParseStream) -> syn::Result<Vec<FixedEntry>> {
    let mut entries = Vec::new();
    while !input.is_empty() {
        entries.push(parse_entry(input)?);
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(entries)
}

fn parse_entry(input: ParseStream) -> syn::Result<FixedEntry> {
    let doc = doc(&input.call(Attribute::parse_outer)?, "a fixed member")?;
    let (name, name_span) = name(input)?;
    if name.is_empty() {
        return Err(syn::Error::new(
            name_span,
            "a fixed member name cannot be empty",
        ));
    }
    input.parse::<Token![:]>()?;
    let ty = input.parse()?;
    input.parse::<Token![=]>()?;
    let value = input.parse()?;
    Ok(FixedEntry {
        name,
        name_span,
        doc,
        ty,
        value,
    })
}

/// Parses a member name: a Rust identifier, or a string literal. The name of a raw identifier has no `r#`. The
/// caller rejects an empty name.
pub(crate) fn name(input: ParseStream) -> syn::Result<(String, Span)> {
    let lookahead = input.lookahead1();
    if lookahead.peek(Ident) {
        let ident: Ident = input.parse()?;
        let name = ident.to_string();
        let name = name.strip_prefix("r#").map(str::to_owned).unwrap_or(name);
        Ok((name, ident.span()))
    } else if lookahead.peek(LitStr) {
        let lit: LitStr = input.parse()?;
        Ok((lit.value(), lit.span()))
    } else {
        Err(lookahead.error())
    }
}

/// The text of the doc comment: the `doc` attribute lines, each with one leading space removed, joined with `\n`
/// and trimmed. `member` names the member in error messages, for example `a fixed member`.
pub(crate) fn doc(attrs: &[Attribute], member: &str) -> syn::Result<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            return Err(syn::Error::new_spanned(
                attr,
                format!("{member} accepts only doc comments"),
            ));
        }
        let value = &attr.meta.require_name_value()?.value;
        let Expr::Lit(ExprLit {
            lit: Lit::Str(line),
            ..
        }) = value
        else {
            return Err(syn::Error::new_spanned(
                value,
                format!("the doc comment of {member} must be a string literal"),
            ));
        };
        let line = line.value();
        lines.push(line.strip_prefix(' ').map(str::to_owned).unwrap_or(line));
    }
    Ok(lines.join("\n").trim().to_owned())
}

/// Makes the `::problem_details_fine::__private::Fixed { name, description, value }` expression for one entry.
pub(crate) fn expand(entry: &FixedEntry) -> syn::Result<TokenStream> {
    let FixedEntry {
        name,
        name_span,
        doc,
        ty,
        value,
    } = entry;
    let mut errors = crate::Errors::default();
    if doc.is_empty() {
        errors.push(syn::Error::new(
            *name_span,
            "a fixed member needs a doc comment; it becomes the description in the Openapi schema",
        ));
    }
    if !is_literal(value) {
        errors.push(syn::Error::new_spanned(
            value,
            "the value of a fixed member must be a literal",
        ));
    }
    let variant = variant(ty);
    if variant.is_none() {
        errors.push(syn::Error::new_spanned(
            ty,
            "unsupported fixed member type; use `&str`, `bool`, an integer type, `f32` or `f64`",
        ));
    }
    errors.finish()?;
    let (variant, cast) = variant.expect("checked above");

    // The constant has the declared type, so rustc reports a mismatch at the value.
    let checked = quote_spanned! {value.span()=>
        {
            const V: #ty = #value;
            V #cast
        }
    };
    Ok(quote! {
        ::problem_details_fine::__private::Fixed {
            name: #name,
            description: #doc,
            value: ::problem_details_fine::__private::FixedValue::#variant(#checked),
        }
    })
}

/// Returns `true` if the value is a literal or a negated literal. It looks through `Expr::Group` and `Expr::Paren`
/// wrappers.
///
/// A `macro_rules!` `$v:expr` fragment, or a parenthesized expression such as `(-1)`, parses as an `Expr::Group` or
/// `Expr::Paren` around the literal. The generated code uses the original `value`, with its wrapper and its span.
/// Only this check looks through the wrapper.
fn is_literal(value: &Expr) -> bool {
    match unwrap(value) {
        Expr::Lit(_) => true,
        Expr::Unary(unary) => {
            matches!(unary.op, UnOp::Neg(_)) && matches!(unwrap(&unary.expr), Expr::Lit(_))
        }
        _ => false,
    }
}

/// Removes all `Expr::Group` and `Expr::Paren` wrappers, and returns the innermost expression. An `Expr::Group` is
/// an invisible delimiter, for example around a `macro_rules!` `$v:expr` substitution.
fn unwrap(mut value: &Expr) -> &Expr {
    loop {
        value = match value {
            Expr::Group(group) => &group.expr,
            Expr::Paren(paren) => &paren.expr,
            _ => return value,
        };
    }
}

/// The `FixedValue` variant for a declared type, and the cast from that type to the variant's type.
fn variant(ty: &Type) -> Option<(Ident, TokenStream)> {
    let (variant, cast) = match ty {
        Type::Reference(reference)
            if reference.mutability.is_none()
                && reference
                    .lifetime
                    .as_ref()
                    .is_none_or(|lifetime| lifetime.ident == "static")
                && matches!(&*reference.elem, Type::Path(path) if path.qself.is_none() && path.path.is_ident("str")) =>
        {
            ("Str", quote!())
        }
        Type::Path(path) if path.qself.is_none() => {
            match path.path.get_ident()?.to_string().as_str() {
                "bool" => ("Bool", quote!()),
                "i8" | "i16" | "i32" | "i64" | "isize" => ("I64", quote!(as i64)),
                "u8" | "u16" | "u32" | "u64" | "usize" => ("U64", quote!(as u64)),
                "f32" => ("F32", quote!()),
                "f64" => ("F64", quote!()),
                _ => return None,
            }
        }
        _ => return None,
    };
    Some((Ident::new(variant, Span::call_site()), cast))
}
