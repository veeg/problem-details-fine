use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{Data, DeriveInput, Fields, Token, Type, punctuated::Punctuated, spanned::Spanned};

use crate::{Errors, attrs};

#[derive(Default)]
struct Args {
    documents: Vec<Type>,
}

impl Args {
    fn parse(tokens: TokenStream) -> syn::Result<Self> {
        let mut args = Self::default();
        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("documents") {
                let content;
                syn::parenthesized!(content in meta.input);
                let types = Punctuated::<Type, Token![,]>::parse_terminated(&content)?;
                args.documents.extend(types);
                Ok(())
            } else {
                Err(meta.error("unknown `problem_set` argument; expected `documents(...)`"))
            }
        });
        syn::parse::Parser::parse2(parser, tokens)?;
        Ok(args)
    }
}

pub(crate) fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args = Args::parse(args)?;
    let input: DeriveInput = syn::parse2(item)?;
    let mut errors = Errors::default();

    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            Span::call_site(),
            "`problem_set` must be applied to an enum whose variants each hold one problem type or problem set",
        ));
    };
    if !input.generics.params.is_empty() {
        errors.push(syn::Error::new_spanned(
            &input.generics,
            "problem sets cannot be generic",
        ));
    }
    if data.variants.is_empty() && args.documents.is_empty() {
        errors.push(syn::Error::new_spanned(
            &input.ident,
            "an empty problem set documents nothing; add variants or `documents(...)`",
        ));
    }

    let mut variants = Vec::new();
    for variant in &data.variants {
        match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                variants.push((&variant.ident, &fields.unnamed[0].ty, attrs::cfgs(&variant.attrs)));
            }
            _ => errors.push(syn::Error::new_spanned(
                variant,
                "a `problem_set` variant must be a tuple variant with one field, and the field must be a problem type or a problem set",
            )),
        }
    }

    // A type that occurs two times in this definition is an error. A type that also occurs in a nested set is
    // correct.
    let mut seen: Vec<(String, &Type)> = Vec::new();
    let members = variants
        .iter()
        .map(|(_, ty, _)| *ty)
        .chain(args.documents.iter());
    for ty in members {
        let key = ty.to_token_stream().to_string();
        if seen.iter().any(|(seen, _)| *seen == key) {
            errors.push(syn::Error::new_spanned(
                ty,
                format!("`{key}` is listed more than once in this problem set"),
            ));
        } else {
            seen.push((key, ty));
        }
    }

    errors.finish()?;

    let ident = &input.ident;
    let variant_idents: Vec<_> = variants.iter().map(|(ident, _, _)| *ident).collect();
    let variant_types: Vec<_> = variants.iter().map(|(_, ty, _)| *ty).collect();
    // The `#[cfg(..)]` of a variant applies to all code that the macro generates for the variant.
    let cfgs: Vec<_> = variants
        .iter()
        .map(|(_, _, cfgs)| quote!(#(#cfgs)*))
        .collect();
    let documents = &args.documents;
    // An error for a variant that is not a problem type points at the type of the variant, not at the attribute.
    let into_details: Vec<_> = variant_types
        .iter()
        .map(|ty| quote_spanned!(ty.span()=> ::problem_details_fine::ProblemDetails::from))
        .collect();

    // Each problem type in the set must have a different type URI. The `problem_set` of each member checks the
    // problem types inside the member. Thus, it is sufficient to compare each pair of members. The error points at
    // the second member of the pair.
    let members: Vec<_> = variants
        .iter()
        .zip(&cfgs)
        .map(|((_, ty, _), cfgs)| (*ty, cfgs.clone()))
        .chain(documents.iter().map(|ty| (ty, TokenStream::new())))
        .collect();
    let mut distinct_members = Vec::new();
    for (j, (second, second_cfgs)) in members.iter().enumerate() {
        let second_name = member_name(second);
        for (first, first_cfgs) in &members[..j] {
            let first_name = member_name(first);
            distinct_members.push(quote_spanned! {second.span()=>
                #first_cfgs
                #second_cfgs
                const _: () = ::problem_details_fine::__private::assert_distinct_members(
                    <#first as ::problem_details_fine::__private::ProblemSetImpl>::__TYPE_URIS,
                    #first_name,
                    <#second as ::problem_details_fine::__private::ProblemSetImpl>::__TYPE_URIS,
                    #second_name,
                );
            });
        }
    }

    Ok(quote! {
        #input

        impl ::problem_details_fine::__private::serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::problem_details_fine::__private::serde::Serializer,
            {
                match *self {
                    #(#cfgs Self::#variant_idents(ref problem) => {
                        ::problem_details_fine::__private::serde::Serialize::serialize(problem, serializer)
                    })*
                }
            }
        }

        impl ::core::convert::From<#ident> for ::problem_details_fine::ProblemDetails {
            fn from(problem: #ident) -> Self {
                match problem {
                    #(#cfgs #ident::#variant_idents(problem) => #into_details(problem),)*
                }
            }
        }

        #(
            #cfgs
            impl ::core::convert::From<#variant_types> for #ident {
                fn from(problem: #variant_types) -> Self {
                    Self::#variant_idents(problem)
                }
            }
        )*

        impl ::problem_details_fine::ProblemSet for #ident {
            fn status(&self) -> ::problem_details_fine::__private::http::StatusCode {
                match *self {
                    #(#cfgs Self::#variant_idents(ref problem) => ::problem_details_fine::ProblemSet::status(problem),)*
                }
            }
        }

        impl ::problem_details_fine::__private::ProblemSetImpl for #ident {

            const __COUNT: usize = {
                let count = 0;
                #(
                    #cfgs
                    let count = count + <#variant_types as ::problem_details_fine::__private::ProblemSetImpl>::__COUNT;
                )*
                #(let count = count + <#documents as ::problem_details_fine::__private::ProblemSetImpl>::__COUNT;)*
                count
            };

            const __TYPE_URIS: &'static [(&'static str, &'static str)] = &{
                let mut types = [("", ""); <#ident as ::problem_details_fine::__private::ProblemSetImpl>::__COUNT];
                let next = 0;
                #(
                    #cfgs
                    let next = ::problem_details_fine::__private::copy_types(
                        &mut types,
                        next,
                        <#variant_types as ::problem_details_fine::__private::ProblemSetImpl>::__TYPE_URIS,
                    );
                )*
                #(
                    let next = ::problem_details_fine::__private::copy_types(
                        &mut types,
                        next,
                        <#documents as ::problem_details_fine::__private::ProblemSetImpl>::__TYPE_URIS,
                    );
                )*
                let _ = next;
                types
            };

            ::problem_details_fine::__if_utoipa! {
                fn __entries(out: &mut ::std::vec::Vec<::problem_details_fine::__private::ProblemEntry>) {
                    #(#cfgs <#variant_types as ::problem_details_fine::__private::ProblemSetImpl>::__entries(out);)*
                    #(<#documents as ::problem_details_fine::__private::ProblemSetImpl>::__entries(out);)*
                }
            }
        }

        #(#distinct_members)*

        ::problem_details_fine::__if_utoipa! {
            impl ::problem_details_fine::__private::utoipa::IntoResponses for #ident {
                fn responses() -> ::std::collections::BTreeMap<
                    ::std::string::String,
                    ::problem_details_fine::__private::utoipa::openapi::RefOr<
                        ::problem_details_fine::__private::utoipa::openapi::response::Response,
                    >,
                > {
                    ::problem_details_fine::__private::problem_responses::<Self>()
                }
            }
        }

        ::problem_details_fine::__if_axum! {
            impl ::problem_details_fine::__private::axum::response::IntoResponse for #ident {
                fn into_response(self) -> ::problem_details_fine::__private::axum::response::Response {
                    match self {
                        #(#cfgs Self::#variant_idents(problem) => {
                            ::problem_details_fine::__private::axum::response::IntoResponse::into_response(problem)
                        })*
                    }
                }
            }
        }
    })
}

/// The name of a member of a problem set in an error message, for example `auth::Problems`.
fn member_name(ty: &Type) -> String {
    ty.to_token_stream()
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}
