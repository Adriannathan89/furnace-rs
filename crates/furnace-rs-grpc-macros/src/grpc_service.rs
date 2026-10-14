//! Protobuf-first service metadata and typed registration expansion.
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Error, Ident, ImplItem, ItemImpl, LitStr, Path, Token, Type,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

struct Arguments {
    server: Path,
    descriptor: Path,
    guard: Option<Path>,
}
impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let (mut server, mut descriptor, mut guard) = (None, None, None);
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let target = match key.to_string().as_str() {
                "server" => &mut server,
                "descriptor" => &mut descriptor,
                "guard" => &mut guard,
                _ => {
                    return Err(Error::new(
                        key.span(),
                        "unknown grpc_service option; expected server, descriptor, or guard",
                    ));
                }
            };
            if target.is_some() {
                return Err(Error::new(key.span(), "duplicate grpc_service option"));
            }
            *target = Some(input.parse::<Path>()?);
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        let server =
            server.ok_or_else(|| input.error("grpc_service requires server = GeneratedServer"))?;
        if server.segments.iter().any(|s| !s.arguments.is_empty()) {
            return Err(Error::new(
                server.span(),
                "server must be a generated constructor path without generic arguments",
            ));
        }
        Ok(Self {
            server,
            descriptor: descriptor.ok_or_else(|| {
                input.error("grpc_service requires descriptor = FILE_DESCRIPTOR_SET")
            })?,
            guard,
        })
    }
}

struct MethodArguments {
    name: LitStr,
    guard: Option<Path>,
}
impl Parse for MethodArguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let (mut name, mut guard, mut public) = (None, None, false);
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            match key.to_string().as_str() {
                "name" => {
                    if name.is_some() {
                        return Err(input.error("duplicate grpc_method name"));
                    }
                    input.parse::<Token![=]>()?;
                    name = Some(input.parse::<LitStr>()?);
                }
                "guard" => {
                    if guard.is_some() {
                        return Err(input.error("duplicate grpc_method guard"));
                    }
                    input.parse::<Token![=]>()?;
                    guard = Some(input.parse::<Path>()?);
                }
                "public" => {
                    if public {
                        return Err(input.error("duplicate grpc_method public option"));
                    }
                    public = true;
                }
                _ => {
                    return Err(Error::new(
                        key.span(),
                        "unknown grpc_method option; expected name, public, or guard",
                    ));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        if public == guard.is_some() {
            return Err(input.error("grpc_method requires exactly one of public or guard"));
        }
        let name: LitStr =
            name.ok_or_else(|| input.error("grpc_method requires the protobuf method name"))?;
        if name.value().is_empty() {
            return Err(Error::new(
                name.span(),
                "protobuf method name must not be empty",
            ));
        }
        Ok(Self { name, guard })
    }
}

pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args: Arguments = syn::parse2(arguments)?;
    let mut implementation: ItemImpl = syn::parse2(item)?;
    if implementation.trait_.is_none()
        || !implementation.generics.params.is_empty()
        || implementation.generics.where_clause.is_some()
        || implementation.unsafety.is_some()
        || implementation.defaultness.is_some()
    {
        return Err(Error::new_spanned(
            &implementation,
            "grpc_service requires a non-generic generated Tonic trait implementation",
        ));
    }
    let Type::Path(self_path) = implementation.self_ty.as_ref() else {
        return Err(Error::new_spanned(
            &implementation.self_ty,
            "grpc_service requires a concrete implementation type",
        ));
    };
    if self_path.qself.is_some() {
        return Err(Error::new_spanned(
            self_path,
            "grpc_service requires a concrete implementation type",
        ));
    }
    let common = crate::path::grpc_path()?;
    let ty = &implementation.self_ty;
    let ident = &self_path.path.segments.last().unwrap().ident;
    let descriptor_ident = format_ident!("__FURNACE_GRPC_{}", ident.unraw());
    let register_ident = format_ident!("__furnace_grpc_register_{}", ident.unraw());
    let guard_value = |guard: &Option<Path>| match guard {
        Some(guard) => quote!(Some(#common::GrpcGuard::of::<#guard>())),
        None => quote!(None),
    };
    let mut overrides = Vec::new();
    let mut rust_methods = Vec::new();
    let mut wire_names = std::collections::HashSet::new();
    for item in &mut implementation.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };
        let rust_name = method.sig.ident.unraw().to_string();
        let method_cfg: Vec<_> = method
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("cfg"))
            .collect();
        rust_methods.push(quote!(#(#method_cfg)* #rust_name));
        let attrs: Vec<_> = method
            .attrs
            .iter()
            .filter(|a| {
                a.path()
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "grpc_method")
            })
            .collect();
        if attrs.len() > 1 {
            return Err(Error::new(
                attrs[1].span(),
                "one grpc_method override is allowed per Rust method",
            ));
        }
        if let Some(attr) = attrs.first() {
            let args: MethodArguments = attr.parse_args()?;
            if !wire_names.insert(args.name.value()) {
                return Err(Error::new(
                    args.name.span(),
                    "duplicate protobuf method override",
                ));
            }
            let name = &args.name;
            let rust_name = method.sig.ident.unraw().to_string();
            let guard = guard_value(&args.guard);
            overrides.push(quote!(#common::GrpcMethodOverride { wire_name: #name, rust_name: #rust_name, guard: #guard }));
        }
        method.attrs.retain(|a| {
            a.path()
                .segments
                .last()
                .is_none_or(|s| s.ident != "grpc_method")
        });
    }
    let server = &args.server;
    let descriptor = &args.descriptor;
    let guard = guard_value(&args.guard);
    let cfg: Vec<_> = implementation
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg"))
        .collect();
    Ok(quote! {
        #implementation
        #(#cfg)*
        #[doc(hidden)]
        #[allow(non_snake_case)]
        fn #register_ident(
            context: &#common::GrpcRegistrationContext<'_>,
            routes: #common::tonic::service::Routes,
        ) -> #common::core::Result<#common::tonic::service::Routes> {
            let implementation = context.application.context().resolve::<#ty>()?;
            let server = #server::from_arc(implementation)
                .max_encoding_message_size(context.config.max_encoding_message_bytes())
                .max_decoding_message_size(context.config.max_decoding_message_bytes());
            context.add_service::<#ty, _>(server, routes)
        }
        #(#cfg)*
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static #descriptor_ident: #common::GrpcServiceDescriptor = #common::GrpcServiceDescriptor::new(
            || ::std::any::TypeId::of::<#ty>(),
            || ::std::any::type_name::<#ty>(),
            || <#server<#ty> as #common::tonic::server::NamedService>::NAME,
            #descriptor, #guard, &[#(#overrides),*],
            #common::core::SourceLocation::new(file!(), line!(), column!()),
        ).with_registrar(#register_ident).with_rust_methods(&[#(#rust_methods),*]);
        #(#cfg)*
        impl #common::core::__private::TransportProvider for #ty {
            const TRANSPORT: &'static str = "grpc";
        }
        #(#cfg)*
        impl #common::GrpcService for #ty {
            fn grpc_descriptor() -> &'static #common::GrpcServiceDescriptor { &#descriptor_ident }
        }
        #(#cfg)*
        #common::core::__private::inventory::submit! { #common::GrpcServiceRegistration { descriptor: &#descriptor_ident } }
    })
}
