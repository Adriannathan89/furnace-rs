//! Function-scoped fixture entry point and Cargo test registration.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ItemFn, Path, spanned::Spanned};

pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !arguments.is_empty() {
        return Err(Error::new(
            arguments.span(),
            "`#[mads::test]` does not accept arguments",
        ));
    }
    let function: ItemFn = syn::parse2(item)
        .map_err(|error| Error::new(error.span(), "`#[mads::test]` requires an async function"))?;
    if function.sig.asyncness.is_none() {
        return Err(Error::new(
            function.sig.fn_token.span(),
            "`#[mads::test]` requires an asynchronous function",
        ));
    }
    if !function.sig.inputs.is_empty() {
        return Err(Error::new(
            function.sig.inputs.span(),
            "`#[mads::test]` does not support function arguments",
        ));
    }
    if !function.sig.generics.params.is_empty() || function.sig.generics.where_clause.is_some() {
        return Err(Error::new(
            function.sig.generics.span(),
            "`#[mads::test]` does not support generic parameters or where clauses",
        ));
    }
    if function.sig.constness.is_some()
        || function.sig.unsafety.is_some()
        || function.sig.abi.is_some()
    {
        return Err(Error::new(
            function.sig.span(),
            "`#[mads::test]` requires a safe, non-const Rust function",
        ));
    }
    Ok(expand_with_path(function, crate::path::testing_path()?))
}

fn expand_with_path(mut function: ItemFn, testing: Path) -> TokenStream {
    let tokio_path = syn::LitStr::new(
        &quote!(#testing::__private::tokio).to_string(),
        function.sig.ident.span(),
    );
    function.block.stmts.insert(
        0,
        syn::parse_quote! {
            fn test_fixture() -> #testing::TestFixtureBuilder {
                #testing::__private::test_fixture()
            }
        },
    );
    quote! {
        #[cfg(test)]
        #[#testing::__private::tokio::test(crate = #tokio_path)]
        #function
    }
}

#[cfg(test)]
#[path = "../tests/support/test_fn.rs"]
mod tests;
