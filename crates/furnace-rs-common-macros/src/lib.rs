//! Procedural macros for managed controllers and inherent HTTP endpoints.
//!
//! A struct `#[controller]` records dependency and seal metadata; an inherent
//! implementation `#[controller(route = "/users")]` emits typed Axum adapters.
//! Static unit-struct guard policies become active only through selected seals.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use proc_macro::TokenStream;

mod controller;
mod endpoint;
mod guard;
mod input;
mod passport_principal;
#[cfg(feature = "passport")]
mod passport_strategy;
mod path;
mod verb;

/// Derives deterministic validation without changing Serde deserialization.
///
/// Use `#[validate(email)]`, length and numeric bounds, `nested`, or synchronous
/// custom callbacks on fields. A complete input supports a custom callback.
#[proc_macro_derive(Input, attributes(validate))]
pub fn input(input: TokenStream) -> TokenStream {
    syn::parse(input)
        .and_then(input::expand)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derives role and permission membership for a named Passport principal.
///
/// Mark at most one collection field with `#[roles]` and at most one with
/// `#[permissions]`. Collection items must implement `AsRef<str>`.
#[proc_macro_derive(PassportPrincipal, attributes(roles, permissions))]
pub fn passport_principal(input: TokenStream) -> TokenStream {
    syn::parse(input)
        .and_then(passport_principal::expand)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Registers a managed, typed Passport JWT strategy implementation.
#[cfg(feature = "passport")]
#[proc_macro_attribute]
pub fn passport_strategy(arguments: TokenStream, item: TokenStream) -> TokenStream {
    passport_strategy::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declares a managed controller struct or its inherent endpoint implementation.
///
/// Annotate the struct with bare `#[controller]`; it is public by default.
/// Implement `Sealable` to protect it, with `#[seal(skip)]` for public endpoints.
/// Annotate exactly one inherent implementation with `#[controller]` or
/// `#[controller(route = "/users")]`; its endpoint methods use HTTP verb
/// attributes. Handlers may be synchronous or asynchronous with `&self` or
/// no receiver. Named fields are resolved as managed dependencies.
#[proc_macro_attribute]
pub fn controller(arguments: TokenStream, item: TokenStream) -> TokenStream {
    controller::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declares a complete static Passport policy on a non-generic unit struct.
///
/// A controller attaches it through `Sealable::seals`; protected endpoints
/// share its strategy, principal, source, role, permission, and predicate rules.
/// Endpoints marked `#[seal(skip)]` bypass that policy.
#[proc_macro_attribute]
pub fn guard(arguments: TokenStream, item: TokenStream) -> TokenStream {
    guard::outside_contract(arguments.into(), item.into()).into()
}

/// Marks one controller endpoint as public with `#[seal(skip)]`.
#[proc_macro_attribute]
pub fn seal(arguments: TokenStream, item: TokenStream) -> TokenStream {
    verb::outside_contract("seal(skip)", arguments.into(), item.into()).into()
}

/// Marks a GET endpoint inside an inherent `#[controller]` implementation.
///
/// Use a bare attribute for the base route or one string path such as
/// `#[get("/:id")]`. Colon captures and wildcards normalize to Axum brace syntax.
#[proc_macro_attribute]
pub fn get(arguments: TokenStream, item: TokenStream) -> TokenStream {
    verb::outside_contract("get", arguments.into(), item.into()).into()
}

/// Marks a POST endpoint, with a bare attribute or one string path.
#[proc_macro_attribute]
pub fn post(arguments: TokenStream, item: TokenStream) -> TokenStream {
    verb::outside_contract("post", arguments.into(), item.into()).into()
}

/// Marks a PUT endpoint, with a bare attribute or one string path.
#[proc_macro_attribute]
pub fn put(arguments: TokenStream, item: TokenStream) -> TokenStream {
    verb::outside_contract("put", arguments.into(), item.into()).into()
}

/// Marks a PATCH endpoint, with a bare attribute or one string path.
#[proc_macro_attribute]
pub fn patch(arguments: TokenStream, item: TokenStream) -> TokenStream {
    verb::outside_contract("patch", arguments.into(), item.into()).into()
}

/// Marks a DELETE endpoint, with a bare attribute or one string path.
#[proc_macro_attribute]
pub fn delete(arguments: TokenStream, item: TokenStream) -> TokenStream {
    verb::outside_contract("delete", arguments.into(), item.into()).into()
}
