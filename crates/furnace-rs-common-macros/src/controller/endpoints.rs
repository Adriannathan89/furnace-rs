//! Expand one inherent endpoint implementation into typed routing metadata.
use crate::endpoint::{self, HandlerAdapter, HttpVerb};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::visit_mut::{self, VisitMut};
use syn::{Error, FnArg, ImplItem, ItemImpl, LitStr, Meta, Token, spanned::Spanned};

struct Arguments {
    route: LitStr,
}
impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self {
                route: LitStr::new("", input.span()),
            });
        }
        let key: syn::Ident = input.parse()?;
        if key != "route" {
            return Err(Error::new(key.span(), "expected `route = \"/...\"`"));
        }
        input.parse::<Token![=]>()?;
        let route = input.parse()?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        if !input.is_empty() {
            return Err(input.error("controller implementations accept only one route prefix"));
        }
        Ok(Self { route })
    }
}

pub(super) fn expand(arguments: TokenStream, mut item: ItemImpl) -> syn::Result<TokenStream> {
    let arguments: Arguments = syn::parse2(arguments)?;
    if item.trait_.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || item.unsafety.is_some()
    {
        return Err(Error::new(
            item.impl_token.span(),
            "endpoint controllers require a non-generic inherent implementation",
        ));
    }
    let common = crate::path::common_path()?;
    let prefix = endpoint::canonical_path(&arguments.route, true)?;
    let controller = &item.self_ty;
    let syn::Type::Path(controller_path) = controller.as_ref() else {
        return Err(Error::new(
            controller.span(),
            "controller implementations require a concrete type path",
        ));
    };
    if controller_path.qself.is_some()
        || controller_path
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
    {
        return Err(Error::new(
            controller.span(),
            "controller implementations require a non-generic concrete type",
        ));
    }
    let ident = &controller_path
        .path
        .segments
        .last()
        .ok_or_else(|| Error::new(controller.span(), "controller type is missing"))?
        .ident;
    let registrar = format_ident!("__furnace_register_endpoints_{ident}");
    let mut descriptors = Vec::new();
    let mut registrations = Vec::new();
    for member in &mut item.items {
        let ImplItem::Fn(method) = member else {
            continue;
        };
        if method.sig.ident.to_string().trim_start_matches("r#") == "seal" {
            return Err(Error::new(
                method.sig.ident.span(),
                "`seal` is reserved for static controller protection",
            ));
        }
        let seal_attributes = method
            .attrs
            .iter()
            .filter(|attr| {
                attr.path()
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "seal")
            })
            .collect::<Vec<_>>();
        if seal_attributes.len() > 1 {
            return Err(Error::new(
                seal_attributes[1].span(),
                "an endpoint can declare `#[seal(skip)]` only once",
            ));
        }
        let seal_skipped = if let Some(attr) = seal_attributes.first() {
            let value = attr.parse_args::<syn::Ident>()?;
            if value != "skip" {
                return Err(Error::new(
                    value.span(),
                    "endpoint seals accept only `#[seal(skip)]`",
                ));
            }
            true
        } else {
            false
        };
        method.attrs.retain(|attr| {
            attr.path()
                .segments
                .last()
                .is_none_or(|segment| segment.ident != "seal")
        });
        let verbs = method
            .attrs
            .iter()
            .enumerate()
            .filter_map(|(i, attr)| endpoint::route_verb(attr).map(|verb| (i, verb)))
            .collect::<Vec<_>>();
        if let Some(attr) = method
            .attrs
            .iter()
            .find(|attribute| crate::guard::is_guard_attribute(attribute))
        {
            return Err(Error::new(
                attr.span(),
                "endpoint policies must be declared through the controller's Sealable implementation",
            ));
        }
        for attr in &method.attrs {
            if attr.path().is_ident("cfg_attr") && endpoint::cfg_attr_contains_route_verb(attr)? {
                return Err(Error::new(
                    attr.span(),
                    "endpoint verbs must be direct attributes; gate the method with `cfg`",
                ));
            }
        }
        if verbs.is_empty() {
            if seal_skipped {
                return Err(Error::new(
                    method.sig.ident.span(),
                    "`#[seal(skip)]` requires an HTTP endpoint",
                ));
            }
            continue;
        }
        if verbs.len() != 1 {
            return Err(Error::new(
                method.sig.ident.span(),
                "each endpoint requires exactly one HTTP verb",
            ));
        }
        let sig = &method.sig;
        if sig.constness.is_some()
            || sig.unsafety.is_some()
            || sig.abi.is_some()
            || sig.variadic.is_some()
            || !sig.generics.params.is_empty()
            || sig.generics.where_clause.is_some()
        {
            return Err(Error::new(
                sig.span(),
                "endpoints cannot be const, unsafe, extern, variadic, or generic",
            ));
        }
        let receiver = matches!(sig.inputs.first(), Some(FnArg::Receiver(_)));
        for argument in &sig.inputs {
            if let FnArg::Receiver(value) = argument
                && (value.reference.is_none()
                    || value.mutability.is_some()
                    || value.colon_token.is_some()
                    || value
                        .reference
                        .as_ref()
                        .is_some_and(|(_, lifetime)| lifetime.is_some()))
            {
                return Err(Error::new(
                    value.span(),
                    "endpoints support only an immutable `&self` receiver or no receiver",
                ));
            }
        }
        endpoint::validate_body_extractor_order(&sig.inputs, &common)?;
        let (index, verb) = verbs[0];
        let attr = method.attrs.remove(index);
        let path = match &attr.meta {
            Meta::Path(_) => LitStr::new("/", attr.span()),
            Meta::List(list) => list.parse_args::<LitStr>()?,
            Meta::NameValue(_) => {
                return Err(Error::new(
                    attr.span(),
                    "HTTP verbs accept a bare attribute or one string path",
                ));
            }
        };
        let path = endpoint::canonical_path(&path, false)?;
        let full_path = endpoint::join_paths(&prefix, &path)?;
        // The full path is checked again to catch captures repeated across joins.
        let full_path = endpoint::canonical_path(&full_path, false)?;
        let conditional: Vec<_> = method
            .attrs
            .iter()
            .filter(|attr| endpoint::is_conditional_attribute(attr))
            .cloned()
            .collect();
        let name = &method.sig.ident;
        let handler = LitStr::new(&name.to_string(), name.span());
        let verb = HttpVerb::from_name(verb);
        let method_token = verb.tokens(&common);
        descriptors.push(quote! {
            #(#conditional)*
            #common::RouteDescriptor::new(#method_token, #prefix, #path, #full_path, #handler,
                #common::core::SourceLocation::new(file!(), line!(), column!()))
                .with_namespace(module_path!()).with_seal_skipped(#seal_skipped)
        });
        let mut types = method
            .sig
            .inputs
            .iter()
            .filter_map(|argument| match argument {
                FnArg::Typed(value) => Some((*value.ty).clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for ty in &mut types {
            SelfNormalizer {
                controller: &controller_path.path,
            }
            .visit_type_mut(ty);
        }
        let args = endpoint::argument_names(&types);
        let self_arg = receiver.then(|| quote!(&__furnace_controller,));
        let call = quote!(<#controller>::#name(#self_arg #(#args,)*));
        let invocation = if method.sig.asyncness.is_some() {
            quote!(#call.await)
        } else {
            call
        };
        registrations.push(
            HandlerAdapter {
                verb,
                handler: &handler,
                argument_types: &types,
                conditional_attributes: &conditional,
                invocation,
                guard_layer: TokenStream::new(),
            }
            .tokens(&common),
        );
    }
    if descriptors.is_empty() {
        return Err(Error::new(
            item.impl_token.span(),
            "annotated controller implementations require at least one endpoint",
        ));
    }
    let conditional: Vec<_> = item
        .attrs
        .iter()
        .filter(|attr| endpoint::is_conditional_attribute(attr))
        .collect();
    Ok(quote! {
        #item
        #(#conditional)*
        const _: () = {
            #[allow(non_snake_case)]
            fn #registrar(
                mut __furnace_router: #common::__private::Router,
                __furnace_runtime: &#common::__private::RouterBuildContext<'_>,
                __furnace_routes: &mut #common::__private::ValidatedRouteIter<'_>,
            ) -> #common::core::Result<#common::__private::Router> {
                let __furnace_controller = __furnace_runtime.application().resolve::<#controller>()?.as_ref().clone();
                #(#registrations)*
                __furnace_routes.finish()?;
                Ok(__furnace_router)
            }
            #common::core::__private::inventory::submit! {
                #common::ControllerEndpointDescriptor::new(
                    concat!(module_path!(), "::", stringify!(#controller)),
                    || ::core::any::TypeId::of::<#controller>(),
                    #common::core::SourceLocation::new(file!(), line!(), column!()),
                    &[#(#descriptors,)*], #registrar,
                ).with_namespace(module_path!()).with_runtime_type_name(|| ::core::any::type_name::<#controller>())
            }
        };
    })
}

struct SelfNormalizer<'a> {
    controller: &'a syn::Path,
}
impl SelfNormalizer<'_> {
    fn normalize(&self, path: &mut syn::Path) {
        if path
            .segments
            .first()
            .is_some_and(|segment| segment.ident == "Self")
        {
            let suffix = path.segments.iter().skip(1).cloned().collect::<Vec<_>>();
            *path = self.controller.clone();
            path.segments.extend(suffix);
        }
    }
}
impl VisitMut for SelfNormalizer<'_> {
    fn visit_type_path_mut(&mut self, value: &mut syn::TypePath) {
        if value.qself.is_none() {
            self.normalize(&mut value.path);
        }
        visit_mut::visit_type_path_mut(self, value);
    }
    fn visit_expr_path_mut(&mut self, value: &mut syn::ExprPath) {
        if value.qself.is_none() {
            self.normalize(&mut value.path);
        }
        visit_mut::visit_expr_path_mut(self, value);
    }
}
