//! Expansion paths that work through the core crate or public facade.

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use syn::{Error, Path, parse_quote};

/// Resolves the core runtime path visible to the macro consumer.
pub(crate) fn core_path() -> syn::Result<Path> {
    if let Ok(found) = crate_name("furnace-rs-core") {
        return found_path(found, false);
    }

    if let Ok(found) = crate_name("furnace-rs") {
        return found_path(found, true);
    }

    if let Ok(found) = crate_name("furnace-rs-socket") {
        return found_path(found, true);
    }

    if let Ok(found) = crate_name("furnace-rs-grpc") {
        return found_path(found, true);
    }

    Err(Error::new(
        Span::call_site(),
        "FURNACE attributes require a dependency on `furnace-rs-core` or `furnace_rs`",
    ))
}

/// Resolves the fixture dev dependency, including renamed packages.
pub(crate) fn testing_path() -> syn::Result<Path> {
    match crate_name("furnace-rs-testing") {
        Ok(FoundCrate::Itself) => Ok(parse_quote!(crate)),
        Ok(FoundCrate::Name(name)) => named_path(&name, false),
        Err(_) => Err(Error::new(
            Span::call_site(),
            "`#[furnace_rs::test]` requires `furnace-rs-testing` as a dev dependency",
        )),
    }
}

/// Resolves the optional persistence package without adding a core dependency.
pub(crate) fn persistence_path() -> syn::Result<Path> {
    match crate_name("furnace-rs-persistence") {
        Ok(FoundCrate::Itself) => Ok(parse_quote!(::furnace_rs_persistence)),
        Ok(FoundCrate::Name(name)) => named_path(&name, false),
        Err(_) => Err(Error::new(
            Span::call_site(),
            "entity-bound storage requires a direct `furnace-rs-persistence` dependency with feature `sea-orm-storage`",
        )),
    }
}

/// Converts Cargo's package-discovery result into the correct runtime path.
fn found_path(found: FoundCrate, facade: bool) -> syn::Result<Path> {
    match (found, facade) {
        (FoundCrate::Itself, false) => Ok(parse_quote!(::furnace_rs_core)),
        (FoundCrate::Itself, true) => Ok(parse_quote!(::furnace_rs::core)),
        (FoundCrate::Name(name), false) => named_path(&name, false),
        (FoundCrate::Name(name), true) => named_path(&name, true),
    }
}

/// Builds a Rust path for a direct dependency, including Cargo name normalization.
fn named_path(name: &str, facade: bool) -> syn::Result<Path> {
    let ident = syn::Ident::new(&name.replace('-', "_"), Span::call_site());

    if facade {
        Ok(parse_quote!(::#ident::core))
    } else {
        Ok(parse_quote!(::#ident))
    }
}

#[cfg(test)]
include!("../tests/support/path.rs");
