//! Registration attributes re-exported by furnace-rs-grpc and the Furnace facade.
mod grpc_client;
mod grpc_reflection;
mod grpc_service;
mod path;
use proc_macro::TokenStream;
/// Connects a managed implementation to generated protobuf server metadata.
#[proc_macro_attribute]
pub fn grpc_service(arguments: TokenStream, item: TokenStream) -> TokenStream {
    grpc_service::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
/// Creates a named DI wrapper around an application-generated client.
#[proc_macro_attribute]
pub fn grpc_client(arguments: TokenStream, item: TokenStream) -> TokenStream {
    grpc_client::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
/// Declares a root-owned Passport guard for optional reflection.
#[proc_macro_attribute]
pub fn grpc_reflection(arguments: TokenStream, item: TokenStream) -> TokenStream {
    grpc_reflection::expand(arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
