//! Declarative cauldron registration.
use crate::path::core_path;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Fields, ItemStruct};
pub(crate) mod contract;
mod declaration;

/// Validates a unit cauldron and emits its registration metadata and typed contracts.
pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    const FORM: &str = "`#[furnace_rs::cauldron]` supports only non-generic unit structs";
    let declaration = syn::parse2::<declaration::Declaration>(arguments)?;
    let item: ItemStruct = syn::parse2(item)?;
    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
        return Err(Error::new_spanned(&item.generics, FORM));
    }
    if !matches!(item.fields, Fields::Unit) {
        return Err(Error::new_spanned(&item.fields, FORM));
    }
    let core = core_path()?;
    let ident = &item.ident;
    let contract = contract::expand(&declaration, &item, &core)?;
    let registration = {
        let d = &declaration;
        let providers = d.providers.iter().map(|p| {
            let output = &p.output;
            match &p.injector {
                Some(injector) => quote!(.provide_with::<#output, #injector>()),
                None => quote!(.provide::<#output>()),
            }
        });
        let controllers = d.controllers.iter().map(|t| quote!(.controller::<#t>()));
        let grpc_services = d
            .grpc_services
            .iter()
            .chain(d.socket_gateways.iter())
            .chain(d.socket_endpoints.iter())
            .map(|t| quote!(.transport_provider::<#t>()));
        let imports = d.imports.iter().map(|t| quote!(.import::<#t>()));
        let exports = d.exports.iter().map(|t| quote!(.export::<#t>()));
        let global = d.global.then(|| quote!(.global()));
        quote! {
            #core::__private::CauldronDefinitionBuilder::new()
                #(#providers)* #(#grpc_services)* #(#controllers)* #(#imports)* #(#exports)* #global
                .into_definition()
        }
    };
    Ok(quote! {
        #item
        #contract
        #core::__private::inventory::submit! {
            #core::CauldronDescriptor::new(
                concat!(module_path!(), "::", stringify!(#ident)),
                || ::core::any::TypeId::of::<#ident>(),
                #core::SourceLocation::new(file!(), line!(), column!()),
            ).with_namespace(module_path!()).with_registration(
                || { #registration }
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::expand;
    use quote::quote;

    #[test]
    fn rejects_nonunit_shapes_before_path_resolution() {
        for (args, item) in [
            (
                quote!(),
                quote!(
                    struct App {
                        field: bool,
                    }
                ),
            ),
            (
                quote!(),
                quote!(
                    struct App(bool);
                ),
            ),
            (
                quote!(),
                quote!(
                    struct App<T>;
                ),
            ),
        ] {
            let error = expand(args, item).unwrap_err();
            assert!(error.to_string().contains("non-generic unit structs"));
        }
    }
}
