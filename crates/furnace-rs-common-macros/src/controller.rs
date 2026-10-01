//! Dispatch controller declarations to managed structs or inherent endpoints.
use proc_macro2::TokenStream;
use syn::{Error, Item};
mod endpoints;
mod managed;

pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    match syn::parse2::<Item>(item.clone())? {
        Item::Struct(value) if arguments.is_empty() => managed::expand_controller_with_common(
            managed::ControllerArguments { routes: Vec::new() },
            value,
            &crate::path::common_path()?,
        ),
        Item::Struct(_) => managed::expand(arguments, item),
        Item::Impl(value) => endpoints::expand(arguments, value),
        other => Err(Error::new_spanned(
            other,
            "`#[controller]` requires a struct or inherent implementation",
        )),
    }
}
