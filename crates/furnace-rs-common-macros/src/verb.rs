//! Guard expansion for endpoint attributes used outside endpoint declarations.
//!
//! The verb attributes are exported as procedural macros, so this guard keeps
//! misuse diagnosable even when a verb appears without a surrounding
//! inherent `#[controller]` implementation for the main route expander to inspect.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Error, spanned::Spanned};

/// Emits a focused diagnostic when an endpoint attribute survives `#[controller]`.
///
/// The original item is preserved in the expansion so rustc can report the
/// focused error at the attribute site without cascading parse failures.
pub(crate) fn outside_contract(
    verb: &str,
    arguments: TokenStream,
    item: TokenStream,
) -> TokenStream {
    let span = if arguments.is_empty() {
        Span::call_site()
    } else {
        arguments.span()
    };
    let error = Error::new(
        span,
        format!(
            "`#[{verb}]` must be used on a method inside an inherent `#[controller]` implementation"
        ),
    )
    .into_compile_error();

    quote!(#item #error)
}
