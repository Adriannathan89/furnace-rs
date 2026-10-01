//! Explicit furnace declarations and fluent chain entry points.
use crate::path::core_path;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Fields, ItemStruct};

pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    const FORM: &str = "`#[mads::furnace]` supports only non-generic unit structs with no arguments; declare imports and global status in `Furnace::register`";
    if !arguments.is_empty() {
        return Err(Error::new_spanned(arguments, FORM));
    }
    let item: ItemStruct = syn::parse2(item)?;
    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
        return Err(Error::new_spanned(&item.generics, FORM));
    }
    if !matches!(item.fields, Fields::Unit) {
        return Err(Error::new_spanned(&item.fields, FORM));
    }
    let core = core_path()?;
    let ident = &item.ident;
    Ok(quote! {
        #item
        impl #ident {
            #[doc = "Starts registration of a local provider output."]
            #[track_caller]
            pub fn provide<T: Send + Sync + 'static>(self) -> #core::FurnaceRegistration<Self> {
                #core::FurnaceRegistration::new(self).provide::<T>()
            }
            #[doc = "Starts registration of a local controller."]
            #[track_caller]
            pub fn controller<T: Send + Sync + 'static>(self) -> #core::FurnaceRegistration<Self> {
                #core::FurnaceRegistration::new(self).controller::<T>()
            }
            #[doc = "Starts registration by importing another furnace."]
            #[track_caller]
            pub fn import<I: #core::Furnace>(self, module: I) -> #core::FurnaceRegistration<Self> {
                #core::FurnaceRegistration::new(self).import(module)
            }
            #[doc = "Starts registration of an exported provider."]
            #[track_caller]
            pub fn export<T: Send + Sync + 'static>(self) -> #core::FurnaceRegistration<Self> {
                #core::FurnaceRegistration::new(self).export::<T>()
            }
            #[doc = "Starts a globally visible furnace declaration."]
            pub fn global(self) -> #core::FurnaceRegistration<Self> {
                #core::FurnaceRegistration::new(self).global()
            }
        }
        #core::__private::inventory::submit! {
            #core::ModuleDescriptor::new(
                concat!(module_path!(), "::", stringify!(#ident)),
                || ::core::any::TypeId::of::<#ident>(),
                #core::SourceLocation::new(file!(), line!(), column!()),
            ).with_namespace(module_path!()).with_registration(
                || <#ident as #core::Furnace>::register(#ident).into_definition()
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::expand;
    use quote::quote;

    #[test]
    fn rejects_legacy_arguments_and_nonunit_shapes_before_path_resolution() {
        for (args, item) in [
            (
                quote!(global),
                quote!(
                    struct App;
                ),
            ),
            (
                quote!(imports = []),
                quote!(
                    struct App;
                ),
            ),
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
            assert!(error.to_string().contains("Furnace::register"));
        }
    }
}
