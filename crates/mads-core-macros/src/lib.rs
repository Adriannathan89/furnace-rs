//! Procedural macros for declaring MADS.rs modules and managed providers.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use proc_macro::TokenStream;

mod configuration;
mod furnace;
#[path = "main.rs"]
mod main_attribute;
mod managed;
mod path;
mod provider;
mod test_fn;

/// Derives an explicit typed view over an already loaded configuration.
///
/// Named structs support `#[config(prefix = "...")]` and field-level
/// `rename`, literal `default`, `parse_with`, and compatible `validate` attributes.
#[proc_macro_derive(Configuration, attributes(config))]
pub fn configuration(input: TokenStream) -> TokenStream {
    syn::parse(input)
        .and_then(configuration::expand)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Replaces an asynchronous application entry point with a Tokio-backed main function.
#[proc_macro_attribute]
pub fn main(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let original = item.clone();
    match main_attribute::expand(arguments.into(), item.into()) {
        Ok(expanded) => expanded.into(),
        Err(error) => {
            let mut output = original;
            output.extend(TokenStream::from(error.into_compile_error()));
            output
        }
    }
}

/// Declares a non-generic unit struct composed through `Furnace::register`.
#[proc_macro_attribute]
pub fn furnace(arguments: TokenStream, item: TokenStream) -> TokenStream {
    furnace::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declares a free function as a general-purpose provider.
#[proc_macro_attribute]
pub fn element(arguments: TokenStream, item: TokenStream) -> TokenStream {
    provider::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declares a named-field or unit struct as an application service.
#[proc_macro_attribute]
pub fn burner(arguments: TokenStream, item: TokenStream) -> TokenStream {
    managed::expand(managed::ManagedKind::Service, arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declares a named-field or unit struct as a persistence repository.
#[proc_macro_attribute]
pub fn storage(arguments: TokenStream, item: TokenStream) -> TokenStream {
    managed::expand(
        managed::ManagedKind::Repository,
        arguments.into(),
        item.into(),
    )
    .unwrap_or_else(syn::Error::into_compile_error)
    .into()
}

/// Registers a zero-argument async Cargo test with a function-local fixture builder.
#[proc_macro_attribute]
pub fn test(arguments: TokenStream, item: TokenStream) -> TokenStream {
    test_fn::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
