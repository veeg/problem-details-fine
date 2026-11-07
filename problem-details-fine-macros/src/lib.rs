//! Procedural macros for [`problem-details-fine`](https://docs.rs/problem-details-fine).
//!
//! Use the macros from `problem_details_fine`. The generated code uses paths that start with
//! `::problem_details_fine`.

use proc_macro::TokenStream;

mod attrs;
mod detail;
mod fixed;
mod problem_set;
mod problem_type;
mod status;
mod uri;

#[proc_macro_attribute]
pub fn problem_type(args: TokenStream, item: TokenStream) -> TokenStream {
    problem_type::expand(args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn problem_set(args: TokenStream, item: TokenStream) -> TokenStream {
    problem_set::expand(args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Registers `serde` and `schema` as helper attributes on a problem type. A problem type does not derive
/// `Serialize`, thus it needs this registration. This derive generates no code.
#[doc(hidden)]
#[proc_macro_derive(__ProblemHelpers, attributes(serde, schema))]
pub fn problem_helpers(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// Collects errors, so that one expansion reports all of them.
#[derive(Default)]
struct Errors(Option<syn::Error>);

impl Errors {
    fn push(&mut self, error: syn::Error) {
        match &mut self.0 {
            Some(errors) => errors.combine(error),
            None => self.0 = Some(error),
        }
    }

    fn finish(self) -> syn::Result<()> {
        self.0.map_or(Ok(()), Err)
    }
}
