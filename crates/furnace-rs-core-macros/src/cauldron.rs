//! Explicit cauldron declarations and fluent chain entry points.
use crate::path::core_path;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Fields, ItemStruct};
pub(crate) mod contract;
mod declaration;

/// Validates a unit cauldron and emits its fluent registration and metadata contracts.
pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    const FORM: &str = "`#[furnace_rs::cauldron]` supports only non-generic unit structs; legacy registration uses `Cauldron::register`";
    let declaration = if arguments.is_empty() {
        None
    } else {
        Some(syn::parse2::<declaration::Declaration>(arguments)?)
    };
    let item: ItemStruct = syn::parse2(item)?;
    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
        return Err(Error::new_spanned(&item.generics, FORM));
    }
    if !matches!(item.fields, Fields::Unit) {
        return Err(Error::new_spanned(&item.fields, FORM));
    }
    let core = core_path()?;
    let ident = &item.ident;
    let contract = declaration
        .as_ref()
        .map(|d| contract::expand(d, &item, &core))
        .transpose()?;
    let registration = declaration.as_ref().map(|d| {
        let providers = d.providers.iter().map(|p| {
            let output = &p.output;
            match &p.injector {
                Some(injector) => quote!(.provide_with::<#output, #injector>()),
                None => quote!(.provide::<#output>()),
            }
        });
        let controllers = d.controllers.iter().map(|t| quote!(.controller::<#t>()));
        let grpc_services = d.grpc_services.iter().map(|t| quote!(.transport_provider::<#t>()));
        let imports = d.imports.iter().map(|t| quote!(.import(#t)));
        let exports = d.exports.iter().map(|t| quote!(.export::<#t>()));
        let global = d.global.then(|| quote!(.global()));
        quote! {
            impl #core::Cauldron for #ident {
                #[doc = "Returns this declaration's static provider, controller, import, and export topology."]
                fn register(self) -> #core::CauldronRegistration<Self> {
                    #core::CauldronRegistration::new(self) #(#providers)* #(#grpc_services)* #(#controllers)* #(#imports)* #(#exports)* #global
                }
            }
        }
    });
    Ok(quote! {
        #item
        #registration
        #contract
        impl #ident {
            #[doc = "Starts registration of a local provider output."]
            #[track_caller]
            pub fn provide<T: #core::Injector>(self) -> #core::CauldronRegistration<Self> {
                #core::CauldronRegistration::new(self).provide::<T>()
            }
            #[doc = "Starts registration of an output using a selected injector."]
            #[track_caller]
            pub fn provide_with<T, I>(self) -> #core::CauldronRegistration<Self>
            where T: Send + Sync + 'static, I: #core::Injector<T> {
                #core::CauldronRegistration::new(self).provide_with::<T, I>()
            }
            #[doc = "Starts registration of a local controller."]
            #[track_caller]
            pub fn controller<T: Send + Sync + 'static>(self) -> #core::CauldronRegistration<Self> {
                #core::CauldronRegistration::new(self).controller::<T>()
            }
            #[doc = "Starts registration by importing another cauldron."]
            #[track_caller]
            pub fn import<I: #core::Cauldron>(self, module: I) -> #core::CauldronRegistration<Self> {
                #core::CauldronRegistration::new(self).import(module)
            }
            #[doc = "Starts registration of an exported provider."]
            #[track_caller]
            pub fn export<T: Send + Sync + 'static>(self) -> #core::CauldronRegistration<Self> {
                #core::CauldronRegistration::new(self).export::<T>()
            }
            #[doc = "Starts a globally visible cauldron declaration."]
            pub fn global(self) -> #core::CauldronRegistration<Self> {
                #core::CauldronRegistration::new(self).global()
            }
        }
        #core::__private::inventory::submit! {
            #core::CauldronDescriptor::new(
                concat!(module_path!(), "::", stringify!(#ident)),
                || ::core::any::TypeId::of::<#ident>(),
                #core::SourceLocation::new(file!(), line!(), column!()),
            ).with_namespace(module_path!()).with_registration(
                || <#ident as #core::Cauldron>::register(#ident).into_definition()
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
            assert!(error.to_string().contains("Cauldron::register"));
        }
    }
}
