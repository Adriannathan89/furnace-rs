//! Typed method adapters and immutable inventory registration.
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::collections::BTreeSet;
use syn::{Expr, ImplItem, ItemImpl, Lit, Meta, Token, parse::Parser, punctuated::Punctuated};

pub(crate) fn expand(
    args: TokenStream,
    item: TokenStream,
    owned: bool,
) -> syn::Result<TokenStream> {
    let socket = crate::path::socket_path()?;
    let mut implementation: ItemImpl = syn::parse2(item)?;

    if implementation.trait_.is_some() || !implementation.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &implementation,
            "socket attributes require a non-generic inherent impl",
        ));
    }

    let mut path = None;
    let mut mode = "events".to_owned();
    let mut concurrent = false;
    let mut limit = 16usize;
    let mut guard = None;
    let mut cookie = "access_token".to_owned();
    let mut query = cookie.clone();
    let mut seen = BTreeSet::new();

    for meta in Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)? {
        let Meta::NameValue(meta) = meta else {
            return Err(syn::Error::new_spanned(meta, "expected name = value"));
        };
        let key = meta.path.to_token_stream().to_string();

        if !seen.insert(key.clone()) {
            return Err(syn::Error::new_spanned(meta, "duplicate socket option"));
        }

        if key == "guard" {
            guard = Some(meta.value);
            continue;
        }

        let Expr::Lit(lit) = &meta.value else {
            return Err(syn::Error::new_spanned(meta, "expected a literal option"));
        };

        match (key.as_str(), &lit.lit) {
            ("path", Lit::Str(v)) => path = Some(v.clone()),
            ("mode", Lit::Str(v)) => mode = v.value(),
            ("processing", Lit::Str(v)) => match v.value().as_str() {
                "sequential" => {}
                "concurrent" => concurrent = true,
                _ => {
                    return Err(syn::Error::new_spanned(
                        v,
                        "expected sequential or concurrent",
                    ));
                }
            },
            ("max_concurrency", Lit::Int(v)) => limit = v.base10_parse()?,
            ("cookie", Lit::Str(v)) => cookie = v.value(),
            ("query", Lit::Str(v)) => query = v.value(),
            _ => {
                return Err(syn::Error::new_spanned(
                    meta,
                    "unknown or invalid socket option",
                ));
            }
        }
    }

    let path =
        path.ok_or_else(|| syn::Error::new_spanned(&implementation, "socket path is required"))?;

    if !path.value().starts_with('/') || path.value().contains(['{', '}', '*', '?', '#']) {
        return Err(syn::Error::new_spanned(
            path,
            "expected a literal absolute endpoint path",
        ));
    }

    if limit == 0 || (!concurrent && seen.contains("max_concurrency")) {
        return Err(syn::Error::new_spanned(
            &implementation,
            "max_concurrency requires concurrent processing and a positive limit",
        ));
    }

    let mode_token = if owned || mode == "raw" {
        quote!(#socket::EndpointMode::Raw)
    } else if mode == "events" {
        quote!(#socket::EndpointMode::Events)
    } else {
        return Err(syn::Error::new_spanned(
            &implementation,
            "mode must be raw or events",
        ));
    };
    let processing = if concurrent {
        quote!(#socket::ProcessingMode::Concurrent{max_concurrency:#limit})
    } else {
        quote!(#socket::ProcessingMode::Sequential)
    };
    let ty = implementation.self_ty.clone();
    let mut records = Vec::new();
    let mut arms = Vec::new();
    let mut names = BTreeSet::new();
    let mut owned_method = None;

    for item in &mut implementation.items {
        let ImplItem::Fn(method) = item else { continue };
        let mut marker = None;
        let mut attrs = Vec::new();

        for attr in std::mem::take(&mut method.attrs) {
            let name = attr
                .path()
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default();

            if name.starts_with("on_") {
                if marker.is_some() {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "one handler attribute per method",
                    ));
                }

                let kind = match name.as_str() {
                    "on_event" => "event",
                    "on_text" => "text",
                    "on_binary" => "binary",
                    "on_connect" => "connect",
                    "on_disconnect" => "disconnect",
                    "on_join" => "join",
                    _ => return Err(syn::Error::new_spanned(attr, "unknown socket handler")),
                };
                let event = if kind == "event" {
                    attr.parse_args::<syn::LitStr>()?.value()
                } else {
                    kind.to_owned()
                };
                marker = Some((kind, event));
            } else {
                attrs.push(attr);
            }
        }

        method.attrs = attrs;

        if !owned && marker.is_none() {
            continue;
        }

        let receiver = method.sig.receiver();

        if method.sig.asyncness.is_none()
            || !method.sig.generics.params.is_empty()
            || receiver.is_none_or(|r| r.reference.is_none() || r.mutability.is_some())
        {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "socket handlers must be async non-generic &self methods",
            ));
        }

        let method_name = &method.sig.ident;

        if owned {
            if owned_method.is_some() || method.sig.inputs.len() != 3 {
                return Err(syn::Error::new_spanned(
                    &method.sig,
                    "owned endpoint requires exactly one method taking handshake and socket",
                ));
            }

            owned_method = Some(method_name.clone());
            continue;
        }

        let (kind, event) = marker.unwrap();
        let count = if matches!(kind, "connect" | "disconnect") {
            2
        } else {
            3
        };

        if method.sig.inputs.len() != count
            || !names.insert((kind.to_owned(), event.clone()))
            || (kind == "event" && (event.is_empty() || event.starts_with("furnace.")))
            || (matches!(kind, "text" | "binary") && mode != "raw")
            || (kind == "event" && mode != "events")
        {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "invalid or duplicate handler for this endpoint mode",
            ));
        }

        let cfg = conditions(&method.attrs)?;
        records.push(quote!(#(#cfg)* #socket::SocketHandlerDescriptor{name:#event,kind:#kind}));
        let input = match kind {
            "event" => quote!(#socket::__private::SocketInput::Event(name,data) if name==#event),
            "text" => quote!(#socket::__private::SocketInput::Text(data)),
            "binary" => quote!(#socket::__private::SocketInput::Binary(data)),
            "join" => quote!(#socket::__private::SocketInput::Join(data)),
            "connect" => quote!(#socket::__private::SocketInput::Connect),
            _ => quote!(#socket::__private::SocketInput::Disconnect),
        };
        let call = if count == 2 {
            quote!(self.#method_name(context))
        } else {
            quote!(self.#method_name(context,data))
        };
        arms.push(quote!(#(#cfg)* #input=>Box::pin(#call),));
    }

    let (trait_impl, route) = if owned {
        let method = owned_method.ok_or_else(|| {
            syn::Error::new_spanned(&implementation, "owned endpoint requires a handler")
        })?;

        (
            quote!(),
            quote!(#socket::__private::owned_route::<#ty>(application, &DESCRIPTOR, |provider,handshake,socket|Box::pin(async move{provider.#method(handshake,socket).await}))),
        )
    } else {
        (
            quote!(impl #socket::__private::SocketGateway for #ty {fn call(&self,context:#socket::SocketContext,input:#socket::__private::SocketInput)->#socket::__private::BoxFuture<'_,#socket::SocketResult<()>>{match input{#(#arms)* #socket::__private::SocketInput::Event(_,_)=>Box::pin(async{Err(#socket::SocketError::new("unknown_event"))}),#socket::__private::SocketInput::Join(_)=>Box::pin(async{Err(#socket::SocketError::new("join_denied"))}),_=>Box::pin(async{Ok(())})}}}),
            quote!(#socket::__private::gateway_route::<#ty>(application,&DESCRIPTOR)),
        )
    };
    let guard = if let Some(guard) = guard {
        quote!(.with_guard(#socket::__private::SocketGuard::of::<#guard>()))
    } else {
        quote!()
    };
    let attrs = conditions(&implementation.attrs)?;

    Ok(quote! {
        #implementation
        #(#attrs)* #trait_impl
        #(#attrs)* impl #socket::core::__private::TransportProvider for #ty {const TRANSPORT:&'static str="socket";}
        #(#attrs)* const _:()={
            static DESCRIPTOR:#socket::SocketEndpointDescriptor=#socket::SocketEndpointDescriptor::new(::std::any::TypeId::of::<#ty>,stringify!(#ty),#path,#mode_token,#processing,&[#(#records),*]).with_location(#socket::core::SourceLocation::new(file!(),line!(),column!())).with_route(route).with_credentials(#cookie,#query) #guard;
            fn route(application:#socket::core::ApplicationContext)->#socket::core::Result<#socket::__private::MethodRouter>{#route}
            #socket::core::__private::inventory::submit!{#socket::SocketEndpointRegistration{descriptor:&DESCRIPTOR}}
        };
    })
}

fn conditions(attributes: &[syn::Attribute]) -> syn::Result<Vec<syn::Attribute>> {
    attributes
        .iter()
        .filter_map(|attribute| condition(&attribute.meta).transpose())
        .map(|result| result.map(|meta| syn::parse_quote!(#[#meta])))
        .collect()
}

fn condition(meta: &Meta) -> syn::Result<Option<Meta>> {
    if meta.path().is_ident("cfg") {
        return Ok(Some(meta.clone()));
    }

    if !meta.path().is_ident("cfg_attr") {
        return Ok(None);
    }

    let Meta::List(list) = meta else {
        return Err(syn::Error::new_spanned(meta, "expected cfg_attr arguments"));
    };
    let mut arguments = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())?
        .into_iter();
    let predicate = arguments
        .next()
        .ok_or_else(|| syn::Error::new_spanned(meta, "cfg_attr requires a predicate"))?;
    let mut nested = Vec::new();

    for argument in arguments {
        if argument
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident.to_string().starts_with("on_"))
        {
            return Err(syn::Error::new_spanned(
                argument,
                "put handler attributes directly on a conditionally compiled method, not inside cfg_attr",
            ));
        }

        if let Some(gate) = condition(&argument)? {
            nested.push(gate);
        }
    }

    if nested.is_empty() {
        Ok(None)
    } else {
        Ok(Some(syn::parse_quote!(cfg_attr(#predicate, #(#nested),*))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn rejects_invalid_public_declarations() {
        for args in [
            quote!(path = "/x", what = "x"),
            quote!(path = "/x", mode = "bad"),
            quote!(path = "/x", processing = "concurrent", max_concurrency = 0),
        ] {
            assert!(expand(args, quote!(impl G{}), false).is_err());
        }
    }

    #[test]
    fn rejects_reserved_events() {
        assert!(
            expand(
                quote!(path = "/x"),
                quote!(impl G{#[on_event("furnace.join")]async fn x(&self,c:C,v:V)->R{todo!()}}),
                false
            )
            .is_err()
        );
    }
}
