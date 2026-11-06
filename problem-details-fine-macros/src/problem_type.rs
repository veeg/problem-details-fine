use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Data, DeriveInput, Expr, Fields, Ident, LitStr, Path, Token, punctuated::Punctuated,
    spanned::Spanned,
};

use crate::{Errors, attrs, detail, status, uri};

/// The members that RFC 9457 and this crate define. An extension member cannot use these names.
const RESERVED: &[&str] = &["type", "status", "title", "detail", "instance"];

#[derive(Default)]
struct Args {
    type_uri: Option<LitStr>,
    status: Option<(u16, Span)>,
    title: Option<LitStr>,
    detail: Option<LitStr>,
    detail_with: Option<Path>,
    tag_uri: Option<LitStr>,
    wrap: Option<LitStr>,
    schema_title: Option<LitStr>,
    payload_schema: Option<Path>,
    /// The `detail` in `omit(..)`.
    omit_detail: Option<Ident>,
    /// The `instance` in `omit(..)`.
    omit_instance: Option<Ident>,
}

impl Args {
    fn parse(tokens: TokenStream) -> syn::Result<Self> {
        let mut args = Self::default();
        let parser = syn::meta::parser(|meta| {
            let path = &meta.path;
            if path.is_ident("type") {
                args.type_uri = Some(meta.value()?.parse()?);
            } else if path.is_ident("status") {
                let expr: Expr = meta.value()?.parse()?;
                args.status = Some((status::parse(&expr)?, expr.span()));
            } else if path.is_ident("title") {
                args.title = Some(meta.value()?.parse()?);
            } else if path.is_ident("detail") {
                args.detail = Some(meta.value()?.parse()?);
            } else if path.is_ident("detail_with") {
                args.detail_with = Some(meta.value()?.parse()?);
            } else if path.is_ident("tag") {
                args.tag_uri = Some(meta.value()?.parse()?);
            } else if path.is_ident("wrap") {
                args.wrap = Some(meta.value()?.parse()?);
            } else if path.is_ident("schema_title") {
                args.schema_title = Some(meta.value()?.parse()?);
            } else if path.is_ident("payload_schema") {
                args.payload_schema = Some(meta.value()?.parse()?);
            } else if path.is_ident("omit") {
                meta.parse_nested_meta(|member| {
                    let slot = if member.path.is_ident("detail") {
                        &mut args.omit_detail
                    } else if member.path.is_ident("instance") {
                        &mut args.omit_instance
                    } else {
                        return Err(member.error("`omit` accepts only `detail` and `instance`"));
                    };
                    let ident = member.path.require_ident()?;
                    if slot.is_some() {
                        return Err(member.error(format!("`{ident}` is already in `omit`")));
                    }
                    *slot = Some(ident.clone());
                    Ok(())
                })?;
            } else {
                return Err(meta.error(
                    "unknown `problem_type` argument; expected one of `type`, `status`, `title`, `detail`, \
                     `detail_with`, `tag`, `wrap`, `schema_title`, `payload_schema`, `omit`",
                ));
            }
            Ok(())
        });
        syn::parse::Parser::parse2(parser, tokens)?;
        Ok(args)
    }
}

enum Layout {
    None,
    Flatten,
    Wrap(LitStr),
}

pub(crate) fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args = Args::parse(args)?;
    let mut input: DeriveInput = syn::parse2(item)?;
    let mut errors = Errors::default();

    let missing = |name: &str| {
        syn::Error::new(
            Span::call_site(),
            format!("`problem_type` requires `{name} = ...`"),
        )
    };
    let type_ = args.type_uri.clone().ok_or_else(|| missing("type"));
    let status = args.status.ok_or_else(|| missing("status"));
    let title = args.title.clone().ok_or_else(|| missing("title"));
    let (type_, status, title) = match (type_, status, title) {
        (Ok(type_), Ok(status), Ok(title)) => (type_, status.0, title),
        (type_, status, title) => {
            for error in [type_.err(), status.err(), title.err()]
                .into_iter()
                .flatten()
            {
                errors.push(error);
            }
            errors.finish()?;
            unreachable!()
        }
    };

    if !input.generics.params.is_empty() {
        errors.push(syn::Error::new_spanned(
            &input.generics,
            "problem types cannot be generic",
        ));
    }
    if let Data::Union(union) = &input.data {
        errors.push(syn::Error::new_spanned(
            union.union_token,
            "`problem_type` supports structs and enums, not unions",
        ));
    }
    for path in derived_serialize(&input.attrs) {
        errors.push(syn::Error::new_spanned(
            path,
            "remove `Serialize` from this derive; `problem_type` implements `Serialize`",
        ));
    }

    // The type URI and the tag URI.
    let type_uri =
        uri::parse_type(&type_.value()).map_err(|message| syn::Error::new_spanned(&type_, message));
    let tag_specific = match (&type_uri, &args.tag_uri) {
        (Ok(uri), Some(tag)) if uri.tag_specific.is_some() => {
            errors.push(syn::Error::new_spanned(
                tag,
                "`tag` is redundant: `type` is already a tag URI and is used as the tag",
            ));
            None
        }
        (_, Some(tag)) => match uri::parse_tag(&tag.value()) {
            Ok(specific) => Some(specific),
            Err(message) => {
                errors.push(syn::Error::new_spanned(tag, message));
                None
            }
        },
        (Ok(uri), None) => uri.tag_specific.clone(),
        (Err(_), None) => None,
    };
    let type_uri = match type_uri {
        Ok(uri) => Some(uri),
        Err(error) => {
            errors.push(error);
            None
        }
    };
    let schema_title = match &args.schema_title {
        Some(title) => title.value(),
        None => tag_specific
            .clone()
            .unwrap_or_else(|| input.ident.to_string()),
    };
    let tag = match (
        &args.tag_uri,
        type_uri.as_ref().and_then(|uri| uri.tag_specific.as_ref()),
    ) {
        (Some(tag), _) => quote!(::core::option::Option::Some(#tag)),
        (None, Some(_)) => quote!(::core::option::Option::Some(#type_)),
        (None, None) => quote!(::core::option::Option::None),
    };

    // The `detail` member.
    if let (Some(detail), Some(_)) = (&args.detail, &args.detail_with) {
        errors.push(syn::Error::new_spanned(
            detail,
            "`detail` and `detail_with` are mutually exclusive",
        ));
    }
    let detail_body = match detail_body(&args, &mut input) {
        Ok(body) => body,
        Err(error) => {
            errors.push(error);
            quote!(::core::option::Option::None)
        }
    };

    // The extension members.
    let layout = match layout(&args, &input) {
        Ok(layout) => layout,
        Err(error) => {
            errors.push(error);
            Layout::None
        }
    };
    let members = match members(&input, &layout) {
        Ok(members) => members,
        Err(error) => {
            errors.push(error);
            Members {
                named: Vec::new(),
                flattened: false,
            }
        }
    };
    if let Err(error) = check_members(&members, &layout, args.tag_uri.as_ref()) {
        errors.push(error);
    }
    let tag_field = tag_field(&members);

    errors.finish()?;

    let ident = input.ident.clone();
    let has_payload = !matches!(layout, Layout::None);
    let extensions = match &layout {
        Layout::None => quote!(::problem_details_fine::__private::Extensions::None),
        Layout::Flatten => quote!(::problem_details_fine::__private::Extensions::Flatten),
        Layout::Wrap(key) => quote!(::problem_details_fine::__private::Extensions::Wrap(#key)),
    };

    // The payload twin is a copy of the item. Through `#[serde(remote)]`, it serializes the extension members of
    // the item. With the `utoipa` feature, it also derives their Openapi schema.
    let twin = has_payload.then(|| {
        let mut twin = input.clone();
        twin.ident = Ident::new("__ProblemPayload", Span::call_site());
        attrs::retain_schema_relevant(&mut twin.attrs);
        match &mut twin.data {
            Data::Struct(data) => data
                .fields
                .iter_mut()
                .for_each(|f| attrs::retain_schema_relevant(&mut f.attrs)),
            Data::Enum(data) => data.variants.iter_mut().for_each(|variant| {
                attrs::retain_schema_relevant(&mut variant.attrs);
                variant
                    .fields
                    .iter_mut()
                    .for_each(|f| attrs::retain_schema_relevant(&mut f.attrs));
            }),
            Data::Union(_) => {}
        }
        let remote = LitStr::new(&ident.to_string(), ident.span());
        let item = quote! {
            #[derive(
                ::problem_details_fine::__private::serde::Serialize,
                ::problem_details_fine::__private::__ProblemHelpers,
            )]
            #[serde(crate = "::problem_details_fine::__private::serde", remote = #remote)]
            #[allow(dead_code, clippy::all)]
            #twin
        };
        match &args.payload_schema {
            // The macro derives the schema only if `payload_schema` is absent.
            None => quote!(::problem_details_fine::__problem_payload! { #item }),
            Some(_) => item,
        }
    });

    let serialize_extensions = match &layout {
        Layout::None => quote!(::problem_details_fine::__private::serialize_no_extensions(
            serializer
        )),
        Layout::Flatten => quote!(__ProblemPayload::serialize(self, serializer)),
        Layout::Wrap(key) => quote! {
            struct __ProblemWrapped<'a>(&'a #ident);
            impl ::problem_details_fine::__private::serde::Serialize for __ProblemWrapped<'_> {
                fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
                where
                    S: ::problem_details_fine::__private::serde::Serializer,
                {
                    __ProblemPayload::serialize(self.0, serializer)
                }
            }
            ::problem_details_fine::__private::serialize_wrapped(serializer, #key, &__ProblemWrapped(self))
        },
    };

    let utoipa = quote!(::problem_details_fine::__private::utoipa);
    let schema_body = match (&args.payload_schema, has_payload) {
        (Some(path), _) => quote! {
            ::problem_details_fine::__private::problem_schema::<#ident>(
                ::core::option::Option::Some(#path()),
                ::std::vec::Vec::new(),
            )
        },
        (None, true) => quote! {
            let mut definitions = ::std::vec::Vec::new();
            <__ProblemPayload as #utoipa::ToSchema>::schemas(&mut definitions);
            ::problem_details_fine::__private::problem_schema::<#ident>(
                ::core::option::Option::Some(<__ProblemPayload as #utoipa::PartialSchema>::schema()),
                definitions,
            )
        },
        (None, false) => quote! {
            ::problem_details_fine::__private::problem_schema::<#ident>(
                ::core::option::Option::None,
                ::std::vec::Vec::new(),
            )
        },
    };
    let ident_str = ident.to_string();
    let omit_detail = args.omit_detail.is_some();
    let omit_instance = args.omit_instance.is_some();

    Ok(quote! {
        #[derive(::problem_details_fine::__private::__ProblemHelpers)]
        #input

        const _: () = {
            #twin

            impl ::problem_details_fine::ProblemType for #ident {
                const TYPE: &'static str = #type_;
                const TAG: ::core::option::Option<&'static str> = #tag;
                const STATUS: ::problem_details_fine::__private::http::StatusCode =
                    ::problem_details_fine::__private::status(#status);
                const TITLE: &'static str = #title;
                const SCHEMA_TITLE: &'static str = #schema_title;

                #[allow(clippy::all, unused_variables)]
                fn detail(&self) -> ::core::option::Option<::std::string::String> {
                    #detail_body
                }
            }

            impl ::problem_details_fine::__private::ProblemTypeImpl for #ident {
                const __EXTENSIONS: ::problem_details_fine::__private::Extensions = #extensions;
                const __TAG_FIELD: ::problem_details_fine::__private::TagField = #tag_field;
                const __OMIT: ::problem_details_fine::__private::Omit = ::problem_details_fine::__private::Omit {
                    detail: #omit_detail,
                    instance: #omit_instance,
                };
                const __ID: &'static str = ::core::concat!(::core::module_path!(), "::", #ident_str);

                fn __serialize_extensions<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
                where
                    S: ::problem_details_fine::__private::serde::Serializer,
                {
                    #serialize_extensions
                }

                fn __resolved() -> &'static ::problem_details_fine::__private::Resolved {
                    static RESOLVED: ::std::sync::OnceLock<::problem_details_fine::__private::Resolved> =
                        ::std::sync::OnceLock::new();
                    RESOLVED.get_or_init(|| {
                        ::problem_details_fine::__private::Resolved::new(
                            <Self as ::problem_details_fine::ProblemType>::TYPE,
                            <Self as ::problem_details_fine::ProblemType>::TAG,
                        )
                    })
                }
            }

            /// Serializes the complete problem details document.
            impl ::problem_details_fine::__private::serde::Serialize for #ident {
                fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
                where
                    S: ::problem_details_fine::__private::serde::Serializer,
                {
                    ::problem_details_fine::__private::serialize_problem(self, serializer)
                }
            }

            ::problem_details_fine::__if_utoipa! {
                impl #utoipa::PartialSchema for #ident {
                    fn schema() -> #utoipa::openapi::RefOr<#utoipa::openapi::schema::Schema> {
                        #schema_body
                    }
                }

                impl #utoipa::ToSchema for #ident {
                    fn name() -> ::std::borrow::Cow<'static, str> {
                        ::std::borrow::Cow::Borrowed(#ident_str)
                    }
                }

                impl #utoipa::IntoResponses for #ident {
                    fn responses() -> ::std::collections::BTreeMap<
                        ::std::string::String,
                        #utoipa::openapi::RefOr<#utoipa::openapi::response::Response>,
                    > {
                        ::problem_details_fine::__private::problem_responses::<Self>()
                    }
                }
            }

            ::problem_details_fine::__if_axum! {
                impl ::problem_details_fine::__private::axum::response::IntoResponse for #ident {
                    fn into_response(self) -> ::problem_details_fine::__private::axum::response::Response {
                        ::problem_details_fine::__private::into_response(
                            <Self as ::problem_details_fine::ProblemType>::STATUS,
                            &self,
                        )
                    }
                }
            }
        };
    })
}

/// Makes the body of `ProblemType::detail`. It removes each `#[problem(..)]` attribute from the item.
fn detail_body(args: &Args, input: &mut DeriveInput) -> syn::Result<TokenStream> {
    let mut errors = Errors::default();

    // Remove the helper attributes first. Then no helper attribute stays in the item, even if a check fails.
    let mut variant_details = Vec::new();
    match &mut input.data {
        Data::Struct(data) => reject_field_helpers(&mut data.fields, &mut errors),
        Data::Enum(data) => {
            for variant in &mut data.variants {
                reject_field_helpers(&mut variant.fields, &mut errors);
                match variant_detail(&mut variant.attrs) {
                    Ok(detail) => variant_details.push(detail),
                    Err(error) => {
                        errors.push(error);
                        variant_details.push(None);
                    }
                }
            }
        }
        Data::Union(_) => {}
    }

    // A type that sets `detail` cannot leave it out of the schema.
    if args.omit_detail.is_some() {
        let message = |source: &str| format!("`omit(detail)` cannot be combined with `{source}`");
        if let Some(detail) = &args.detail {
            errors.push(syn::Error::new_spanned(detail, message("detail")));
        }
        if let Some(path) = &args.detail_with {
            errors.push(syn::Error::new_spanned(path, message("detail_with")));
        }
        for detail in variant_details.iter().flatten() {
            errors.push(syn::Error::new_spanned(
                detail,
                message("#[problem(detail = ..)]"),
            ));
        }
    }

    if let Some(path) = &args.detail_with {
        for detail in variant_details.iter().flatten() {
            errors.push(syn::Error::new_spanned(
                detail,
                "`#[problem(detail = ..)]` cannot be combined with `detail_with`",
            ));
        }
        errors.finish()?;
        return Ok(quote!(#path(self)));
    }

    let body = match &input.data {
        Data::Struct(data) => match &args.detail {
            Some(format) => match detail::render(format, &data.fields) {
                Ok((pattern, expr)) => quote! {
                    let Self #pattern = self;
                    #expr
                },
                Err(error) => {
                    errors.push(error);
                    quote!(::core::option::Option::None)
                }
            },
            None => quote!(::core::option::Option::None),
        },
        Data::Enum(data) => {
            let mut arms = Vec::new();
            for (variant, variant_detail) in data.variants.iter().zip(&variant_details) {
                let ident = &variant.ident;
                let cfgs = attrs::cfgs(&variant.attrs);
                let arm = match variant_detail.as_ref().or(args.detail.as_ref()) {
                    Some(format) => match detail::render(format, &variant.fields) {
                        Ok((pattern, expr)) => quote!(#(#cfgs)* Self::#ident #pattern => #expr,),
                        Err(error) => {
                            errors.push(error);
                            continue;
                        }
                    },
                    None => {
                        let pattern = match &variant.fields {
                            Fields::Named(_) => quote!({ .. }),
                            Fields::Unnamed(_) => quote!((..)),
                            Fields::Unit => quote!(),
                        };
                        quote!(#(#cfgs)* Self::#ident #pattern => ::core::option::Option::None,)
                    }
                };
                arms.push(arm);
            }
            quote! {
                match self {
                    #(#arms)*
                }
            }
        }
        Data::Union(_) => quote!(::core::option::Option::None),
    };
    errors.finish()?;
    Ok(body)
}

/// The paths in `#[derive(..)]` that name `Serialize`, for example `Serialize` or `serde::Serialize`. The compiler
/// reports a `#[derive(..)]` that does not parse.
fn derived_serialize(attrs: &[syn::Attribute]) -> Vec<Path> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .filter(|path| {
            path.segments
                .last()
                .is_some_and(|segment| segment.ident == "Serialize")
        })
        .collect()
}

/// Rejects `#[problem(..)]` on a field. The attribute is correct only on an enum variant.
fn reject_field_helpers(fields: &mut Fields, errors: &mut Errors) {
    for field in fields.iter_mut() {
        for attr in attrs::take(&mut field.attrs, "problem") {
            errors.push(syn::Error::new_spanned(
                attr,
                "`#[problem(..)]` is only supported on enum variants",
            ));
        }
    }
}

/// Removes `#[problem(detail = "...")]` from the attributes of a variant, and returns the format string.
fn variant_detail(attrs: &mut Vec<syn::Attribute>) -> syn::Result<Option<LitStr>> {
    let mut detail = None;
    for attr in attrs::take(attrs, "problem") {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("detail") {
                detail = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("unknown `problem` variant argument; expected `detail`"))
            }
        })?;
    }
    Ok(detail)
}

fn layout(args: &Args, input: &DeriveInput) -> syn::Result<Layout> {
    let has_fields = match &input.data {
        Data::Struct(data) => !data.fields.is_empty(),
        Data::Enum(_) => true,
        Data::Union(_) => return Ok(Layout::None),
    };

    let serde = attrs::container(&input.attrs)?;

    if let Some(opaque @ ("into" | "remote")) = serde.opaque {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("`#[serde({opaque} = ..)]` is not supported on problem types"),
        ));
    }
    if let (Some(opaque), None) = (serde.opaque, &args.payload_schema) {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!(
                "with `#[serde({opaque})]` the derived Openapi schema would describe the fields rather than what \
                 is serialized; provide the schema with `payload_schema = path::to::fn`"
            ),
        ));
    }
    if let Some(wrap) = &args.wrap {
        if !has_fields {
            return Err(syn::Error::new_spanned(
                wrap,
                "`wrap` requires extension members, but this problem type has no fields",
            ));
        }
        if RESERVED.contains(&wrap.value().as_str()) {
            return Err(syn::Error::new_spanned(
                wrap,
                format!(
                    "`wrap = \"{}\"` collides with a reserved problem details member",
                    wrap.value()
                ),
            ));
        }
        return Ok(Layout::Wrap(wrap.clone()));
    }
    if let Some(opaque) = serde.opaque {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!(
                "with `#[serde({opaque})]` the serialized value is not this type's own members, so it cannot be \
                 flattened into the problem details document; nest it with `wrap = \"<member>\"`"
            ),
        ));
    }
    if !has_fields {
        return Ok(Layout::None);
    }

    match &input.data {
        Data::Struct(data) if matches!(data.fields, Fields::Unnamed(_)) => {
            Err(syn::Error::new_spanned(
                &input.ident,
                "tuple structs serialize as a value, not as members, and cannot be flattened into the problem \
             details document; name the fields, or nest the value with `wrap = \"<member>\"`",
            ))
        }
        Data::Enum(data) => {
            if data.variants.is_empty() {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "a problem type enum needs at least one variant",
                ));
            }
            if serde.tag.is_none() {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "enum extension members can only be flattened when the enum serializes as a map; add \
                     `#[serde(tag = \"<member>\")]`, or nest the value with `wrap = \"<member>\"`",
                ));
            }
            if serde.content.is_none() {
                for variant in &data.variants {
                    if matches!(variant.fields, Fields::Unnamed(_)) {
                        return Err(syn::Error::new_spanned(
                            variant,
                            "with `#[serde(tag = ..)]` alone, a tuple variant only serializes when it holds a \
                             struct or map, which cannot be checked here; use named fields, add \
                             `#[serde(content = \"<member>\")]`, or nest the value with `wrap = \"<member>\"`",
                        ));
                    }
                }
            }
            Ok(Layout::Flatten)
        }
        _ => Ok(Layout::Flatten),
    }
}

/// The extension members at the top level of the document, with their serialized names.
struct Members {
    /// The members that the macro can see, with the item that each one comes from.
    named: Vec<(String, TokenStream)>,
    /// `true` if a `#[serde(flatten)]` field adds members that the macro cannot see.
    flattened: bool,
}

/// Returns the extension members at the top level of the document.
fn members(input: &DeriveInput, layout: &Layout) -> syn::Result<Members> {
    let mut members = Members {
        named: Vec::new(),
        flattened: false,
    };
    match layout {
        Layout::None => return Ok(members),
        Layout::Wrap(key) => {
            members.named.push((key.value(), key.to_token_stream()));
            return Ok(members);
        }
        Layout::Flatten => {}
    }
    let container = attrs::container(&input.attrs)?;

    let mut fields = |fields: &Fields, rename_all: Option<&LitStr>| -> syn::Result<()> {
        for field in fields {
            let Some(ident) = &field.ident else { continue };
            let serde = attrs::member(&field.attrs)?;
            if serde.skip {
                continue;
            }
            if serde.flatten {
                members.flattened = true;
                continue;
            }
            let name = match (&serde.rename, rename_all) {
                (Some(rename), _) => rename.value(),
                (None, Some(rule)) => attrs::rename_field(rule, &ident.unraw_string())?,
                (None, None) => ident.unraw_string(),
            };
            members.named.push((name, field.to_token_stream()));
        }
        Ok(())
    };

    match &input.data {
        Data::Struct(data) => fields(&data.fields, container.rename_all.as_ref())?,
        Data::Enum(_) if container.content.is_some() => {
            // An adjacently tagged enum puts the members of the variant in the `content` member.
            for key in [&container.tag, &container.content].into_iter().flatten() {
                members.named.push((key.value(), key.to_token_stream()));
            }
        }
        Data::Enum(data) => {
            for variant in &data.variants {
                let serde = attrs::member(&variant.attrs)?;
                let rename_all = serde
                    .rename_all
                    .as_ref()
                    .or(container.rename_all_fields.as_ref());
                fields(&variant.fields, rename_all)?;
            }
            if let Some(tag) = &container.tag {
                members.named.push((tag.value(), tag.to_token_stream()));
            }
        }
        Data::Union(_) => {}
    }
    Ok(members)
}

/// Rejects an extension member at the top level of the document if its name is a reserved name. Also rejects a
/// `tag` member if the `tag` argument adds the `tag` member of the document.
fn check_members(
    members: &Members,
    layout: &Layout,
    tag_argument: Option<&LitStr>,
) -> syn::Result<()> {
    let mut errors = Errors::default();
    for (name, span) in &members.named {
        if RESERVED.contains(&name.as_str()) && !matches!(layout, Layout::Wrap(_)) {
            errors.push(syn::Error::new_spanned(
                span,
                format!(
                    "the extension member `{name}` collides with a reserved problem details member; rename it \
                     with `#[serde(rename = \"...\")]` or nest the members with `wrap = \"<member>\"`"
                ),
            ));
        }
        if name == "tag"
            && let Some(tag_argument) = tag_argument
        {
            let mut error = syn::Error::new_spanned(
                span,
                "the extension member `tag` collides with the `tag` member that the `tag` argument adds",
            );
            error.combine(syn::Error::new_spanned(
                tag_argument,
                "the `tag` argument is here",
            ));
            errors.push(error);
        }
    }
    errors.finish()
}

/// The `__TAG_FIELD` value: whether the extension members include a `tag` member.
fn tag_field(members: &Members) -> TokenStream {
    if members.named.iter().any(|(name, _)| name == "tag") {
        quote!(::problem_details_fine::__private::TagField::Yes)
    } else if members.flattened {
        quote!(::problem_details_fine::__private::TagField::Unknown)
    } else {
        quote!(::problem_details_fine::__private::TagField::No)
    }
}

trait UnrawString {
    fn unraw_string(&self) -> String;
}

impl UnrawString for Ident {
    fn unraw_string(&self) -> String {
        let name = self.to_string();
        name.strip_prefix("r#").map(str::to_owned).unwrap_or(name)
    }
}
