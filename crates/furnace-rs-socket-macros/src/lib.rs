//! Static WebSocket gateway and owned-endpoint attributes for Furnace.
mod gateway;
mod path;
use proc_macro::TokenStream;
/// Registers an injected gateway and its lifecycle/message methods.
#[proc_macro_attribute]
pub fn socket_gateway(args: TokenStream, item: TokenStream) -> TokenStream {
    gateway::expand(args.into(), item.into(), false)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
/// Registers an injected implementation with one owned-socket method.
#[proc_macro_attribute]
pub fn socket_endpoint(args: TokenStream, item: TokenStream) -> TokenStream {
    gateway::expand(args.into(), item.into(), true)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
