//! Procedural macros for declaring furnace-rs cauldrons and managed providers.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use proc_macro::TokenStream;

mod cauldron;
mod configuration;
#[path = "main.rs"]
mod main_attribute;
mod managed;
mod path;
mod storage;
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

/// Declares a unit cauldron with declarative registration.
///
/// Bare attributes register an empty cauldron. Options declare providers,
/// controllers, imports, exports and global status; `root` checks the reachable
/// declarative graph's typed dependencies during compilation.
#[proc_macro_attribute]
pub fn cauldron(arguments: TokenStream, item: TokenStream) -> TokenStream {
    cauldron::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Internal callback that resolves imported metadata macro identities.
#[doc(hidden)]
#[proc_macro]
pub fn __di_bootstrap(input: TokenStream) -> TokenStream {
    cauldron::contract::bootstrap(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Internal callback that collects a declared root graph and emits checks.
#[doc(hidden)]
#[proc_macro]
pub fn __di_collect(input: TokenStream) -> TokenStream {
    cauldron::contract::collect(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declares a named-field or unit struct as an application service.
/// Every named dependency field requires parameterless `#[inject]`.
#[proc_macro_attribute]
pub fn burner(arguments: TokenStream, item: TokenStream) -> TokenStream {
    managed::expand(managed::ManagedKind::Service, arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declares a named-field or unit struct as a persistence repository.
/// Every named dependency field requires parameterless `#[inject]`.
///
/// Optional `entity = path::Entity` binds native SeaORM CRUD through `entity()`.
/// `connection = field` selects an injected native connection when inference is
/// ambiguous or an alias is used. `relations(alias = selector, ...)` generates
/// explicit native relation loader methods. Entity options require a direct
/// `furnace-rs-persistence` dependency with its `sea-orm-storage` feature.
#[proc_macro_attribute]
pub fn storage(arguments: TokenStream, item: TokenStream) -> TokenStream {
    storage::expand(arguments.into(), item.into())
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
