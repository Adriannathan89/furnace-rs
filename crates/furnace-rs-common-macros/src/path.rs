//! Expansion paths that work through the common crate or public facade.
//!
//! Procedural macros may be invoked through either `furnace-rs-common` directly or
//! the `furnace_rs` facade. This module resolves the path that generated code should
//! use so renamed dependencies and in-crate macro tests remain valid.

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use syn::{Error, Path, parse_quote};

/// Resolves the common integration path visible to the macro consumer.
///
/// Direct users receive a path to `furnace-rs-common`; facade users receive the
/// facade's `common` module. An actionable compile-time error is returned when
/// neither package is present in the consumer's dependency graph.
pub(crate) fn common_path() -> syn::Result<Path> {
    if let Ok(found) = crate_name("furnace-rs-common") {
        return found_path(found, false);
    }

    if let Ok(found) = crate_name("furnace-rs") {
        return found_path(found, true);
    }

    Err(Error::new(
        Span::call_site(),
        "FURNACE common attributes require a dependency on `furnace-rs-common` or `furnace_rs` with the `common` feature",
    ))
}

fn found_path(found: FoundCrate, facade: bool) -> syn::Result<Path> {
    match (found, facade) {
        (FoundCrate::Itself, false) => Ok(parse_quote!(crate)),
        (FoundCrate::Itself, true) => Ok(parse_quote!(::furnace_rs::common)),
        (FoundCrate::Name(name), false) => named_path(&name, false),
        (FoundCrate::Name(name), true) => named_path(&name, true),
    }
}

fn named_path(name: &str, facade: bool) -> syn::Result<Path> {
    let ident = syn::Ident::new(&name.replace('-', "_"), Span::call_site());
    if facade {
        Ok(parse_quote!(::#ident::common))
    } else {
        Ok(parse_quote!(::#ident))
    }
}
#[cfg(test)]
include!("../tests/support/path.rs");
