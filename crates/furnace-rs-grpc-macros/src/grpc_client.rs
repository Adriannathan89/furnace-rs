//! Generates independently named DI wrappers around protobuf-generated clients.
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Error, Fields, Ident, ItemStruct, LitStr, Path, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

struct Arguments {
    name: LitStr,
    client: Path,
    provider: Option<Path>,
}

impl Parse for Arguments {
    /// Parses named-client attribute arguments and rejects duplicates or unknown keys.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let (mut name, mut client, mut provider) = (None, None, None);

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "name" => {
                    if name.is_some() {
                        return Err(input.error("duplicate grpc_client name"));
                    }

                    name = Some(input.parse()?);
                }
                "client" => {
                    if client.is_some() {
                        return Err(input.error("duplicate grpc_client client"));
                    }

                    client = Some(input.parse()?);
                }
                "token_provider" => {
                    if provider.is_some() {
                        return Err(input.error("duplicate grpc_client token_provider"));
                    }

                    provider = Some(input.parse()?);
                }
                _ => {
                    return Err(Error::new(
                        key.span(),
                        "unknown grpc_client option; expected name, client, or token_provider",
                    ));
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            name: name.ok_or_else(|| input.error("grpc_client requires name"))?,
            client: client
                .ok_or_else(|| input.error("grpc_client requires client = GeneratedClient"))?,
            provider,
        })
    }
}

/// Validates a client wrapper declaration and emits its DI and lifecycle integration.
pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args: Arguments = syn::parse2(arguments)?;
    let item: ItemStruct = syn::parse2(item)?;

    if !matches!(item.fields, Fields::Unit)
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
    {
        return Err(Error::new(
            item.span(),
            "grpc_client requires a non-generic unit struct",
        ));
    }

    let name_value = args.name.value();

    if !name_value
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_lowercase())
        || !name_value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
    {
        return Err(Error::new(
            args.name.span(),
            "grpc_client name must start with a lowercase ASCII letter and contain only [a-z0-9_-]",
        ));
    }

    if args.client.segments.iter().any(|s| !s.arguments.is_empty()) {
        return Err(Error::new(
            args.client.span(),
            "grpc_client client must be an unparameterized generated client path",
        ));
    }

    let common = crate::path::grpc_path()?;
    let ty = &item.ident;
    let name = &args.name;
    let client = &args.client;
    let visibility = &item.vis;
    let attrs = &item.attrs;
    let cfg: Vec<_> = attrs.iter().filter(|a| a.path().is_ident("cfg")).collect();
    let (dependencies, pattern, provider) = match args.provider {
        Some(provider) => (
            quote!((#common::core::Config, #provider)),
            quote!((config, provider)),
            quote!(Some(#common::ErasedTokenProvider::of(provider))),
        ),
        None => (
            quote!((#common::core::Config,)),
            quote!((config,)),
            quote!(None),
        ),
    };
    let descriptor = format_ident!("__FURNACE_GRPC_CLIENT_{}", ty);

    Ok(quote! {
        #(#attrs)*
        #[derive(Clone)]
        #visibility struct #ty { channel: #common::GrpcChannel, encoding_limit: usize, decoding_limit: usize }
        #(#cfg)*
        impl #ty {
            /// Returns an independent generated client sharing this named transport.
            #visibility fn client(&self) -> #client<#common::GrpcChannel> {
                #client::new(self.channel.clone()).max_encoding_message_size(self.encoding_limit).max_decoding_message_size(self.decoding_limit)
            }
        }
        #(#cfg)*
        impl #common::core::Injector for #ty {
            type Dependencies = #dependencies;
            #[doc = "Builds the managed named client from application configuration and dependencies."]
            async fn inject(#pattern: Self::Dependencies) -> #common::core::Result<Self> {
                let options = #common::GrpcClientConfig::parse(&config, #name)?;
                let encoding_limit = options.max_encoding_message_bytes(); let decoding_limit = options.max_decoding_message_bytes();
                let channel = #common::GrpcChannel::from_config(options, #provider).await?;
                Ok(Self { channel, encoding_limit, decoding_limit })
            }
            #[doc = "Attaches the managed channel's startup and shutdown lifecycle hook."]
            fn lifecycle(value: Self) -> #common::core::LifecycleResource<Self> {
                let hook = #common::GrpcClientLifecycle::new(value.channel.clone(), #name);
                #common::core::LifecycleResource::new(value).with_infrastructure_hook("furnace.common.grpc.clients", hook)
            }
        }
        #(#cfg)*
        #[allow(non_upper_case_globals)]
        static #descriptor: #common::GrpcClientDescriptor = #common::GrpcClientDescriptor::new(#name, ::std::any::TypeId::of::<#ty>, ::std::any::type_name::<#ty>, #common::core::SourceLocation::new(file!(), line!(), column!()));
        #(#cfg)*
        #common::core::__private::inventory::submit! { #common::GrpcClientRegistration { descriptor: &#descriptor } }
    })
}
