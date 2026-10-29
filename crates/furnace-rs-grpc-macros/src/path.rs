//! Resolves standalone, renamed, and facade transport dependencies.
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use syn::{Error, Path, parse_quote};

/// Resolves the crate path used by generated gRPC code.
pub(crate) fn grpc_path() -> syn::Result<Path> {
    if let Ok(found) = crate_name("furnace-rs-grpc") {
        return match found {
            FoundCrate::Itself => Ok(parse_quote!(crate)),
            FoundCrate::Name(name) => {
                let name = syn::Ident::new(&name.replace('-', "_"), Span::call_site());

                Ok(parse_quote!(::#name))
            }
        };
    }

    if let Ok(found) = crate_name("furnace-rs") {
        let name = match found {
            FoundCrate::Itself => syn::Ident::new("furnace_rs", Span::call_site()),
            FoundCrate::Name(name) => syn::Ident::new(&name.replace('-', "_"), Span::call_site()),
        };

        return Ok(parse_quote!(::#name::grpc));
    }

    Err(Error::new(
        Span::call_site(),
        "gRPC attributes require furnace-rs-grpc or the furnace-rs facade with grpc enabled",
    ))
}
