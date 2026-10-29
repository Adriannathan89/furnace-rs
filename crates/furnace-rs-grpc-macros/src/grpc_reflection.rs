//! Registers a root-owned typed policy for optional native reflection services.
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Error, Ident, ItemStruct, Path, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

struct Arguments {
    guard: Path,
}

impl Parse for Arguments {
    /// Parses the required reflection guard attribute argument.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: Ident = input.parse()?;

        if key != "guard" {
            return Err(Error::new(
                key.span(),
                "grpc_reflection requires guard = GuardPolicy",
            ));
        }

        input.parse::<Token![=]>()?;
        let guard = input.parse()?;

        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        if !input.is_empty() {
            return Err(input.error("grpc_reflection accepts exactly one guard"));
        }

        Ok(Self { guard })
    }
}

/// Validates the reflection provider declaration and emits its registration.
pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args: Arguments = syn::parse2(arguments)?;
    let item: ItemStruct = syn::parse2(item)?;

    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
        return Err(Error::new(
            item.span(),
            "grpc_reflection requires a non-generic cauldron root",
        ));
    }

    let common = crate::path::grpc_path()?;
    let root = &item.ident;
    let guard = args.guard;
    let name = format_ident!("__FURNACE_GRPC_REFLECTION_{}", root);
    let cfg: Vec<_> = item
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg"))
        .collect();

    Ok(quote! {
        #item
        #(#cfg)*
        #[allow(non_upper_case_globals)]
        static #name: #common::GrpcReflectionDescriptor = #common::GrpcReflectionDescriptor::new(::std::any::TypeId::of::<#root>, #common::GrpcGuard::of::<#guard>(), #common::core::SourceLocation::new(file!(), line!(), column!()));
        #(#cfg)*
        #common::core::__private::inventory::submit! { #common::GrpcReflectionRegistration { descriptor: &#name } }
    })
}
