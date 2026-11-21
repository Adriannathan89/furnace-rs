//! Parsing and wiring for the `#[cache]` and `#[rate_limit]` endpoint attributes.
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Attribute;
#[cfg(feature = "extra")]
use syn::{Error, Expr, ExprLit, Lit, Meta, MetaNameValue, Token, punctuated::Punctuated};

/// Settings parsed from one endpoint's extension attributes.
#[derive(Default)]
pub(super) struct EndpointExtras {
    /// `ttl` seconds and the attribute's span.
    pub(super) cache: Option<(u64, Span)>,
    /// `(limit, per)` seconds.
    pub(super) rate_limit: Option<(u64, u64)>,
}

impl EndpointExtras {
    pub(super) fn is_empty(&self) -> bool {
        self.cache.is_none() && self.rate_limit.is_none()
    }
}

/// Removes `cache`/`rate_limit` attributes from a method and parses them.
///
/// Without the `extra` feature nothing is touched, so an application's own
/// attributes with these names keep working.
#[cfg(not(feature = "extra"))]
pub(super) fn take_extras(_: &mut Vec<Attribute>) -> syn::Result<EndpointExtras> {
    Ok(EndpointExtras::default())
}

/// Removes `cache`/`rate_limit` attributes from a method and parses them.
#[cfg(feature = "extra")]
pub(super) fn take_extras(attrs: &mut Vec<Attribute>) -> syn::Result<EndpointExtras> {
    let named = |attr: &Attribute, name: &str| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name)
    };
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(attrs)
        .into_iter()
        .partition(|attr| named(attr, "cache") || named(attr, "rate_limit"));
    *attrs = rest;

    let mut extras = EndpointExtras::default();
    for attr in &mine {
        if named(attr, "cache") {
            if extras.cache.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    "an endpoint can declare `#[cache]` only once",
                ));
            }
            extras.cache = Some((
                positive_args(attr, &["ttl"])?[0],
                syn::spanned::Spanned::span(attr),
            ));
        } else {
            if extras.rate_limit.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    "an endpoint can declare `#[rate_limit]` only once",
                ));
            }
            let values = positive_args(attr, &["limit", "per"])?;
            extras.rate_limit = Some((values[0], values[1]));
        }
    }
    Ok(extras)
}

/// Parses `key = <positive integer literal>` pairs; every key is required, none repeats.
#[cfg(feature = "extra")]
fn positive_args(attr: &Attribute, keys: &[&str]) -> syn::Result<Vec<u64>> {
    if !matches!(attr.meta, Meta::List(_)) {
        return Err(Error::new_spanned(
            attr,
            format!("expected arguments: {}", keys.join(", ")),
        ));
    }
    let pairs = attr.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)?;
    let mut found: Vec<Option<u64>> = vec![None; keys.len()];
    for pair in &pairs {
        let name = pair
            .path
            .get_ident()
            .map(ToString::to_string)
            .unwrap_or_default();
        let Some(index) = keys.iter().position(|key| *key == name) else {
            return Err(Error::new_spanned(
                &pair.path,
                format!("unknown key; expected {}", keys.join(", ")),
            ));
        };
        if found[index].is_some() {
            return Err(Error::new_spanned(&pair.path, "duplicate key"));
        }
        found[index] = Some(positive_literal(&pair.value)?);
    }
    keys.iter()
        .zip(found)
        .map(|(key, value)| {
            value.ok_or_else(|| Error::new_spanned(attr, format!("missing `{key}`")))
        })
        .collect()
}

#[cfg(feature = "extra")]
fn positive_literal(expr: &Expr) -> syn::Result<u64> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(int), ..
        }) => int
            .base10_parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| Error::new_spanned(expr, "expected a positive integer literal")),
        _ => Err(Error::new_spanned(
            expr,
            "expected a positive integer literal",
        )),
    }
}

/// Emits `.route_layer(..)` calls: cache first so the rate limiter wraps it.
///
/// `route` is an expression yielding the endpoint's `&str` identity.
pub(super) fn layer_tokens(
    extras: &EndpointExtras,
    common: &syn::Path,
    route: &TokenStream,
) -> syn::Result<TokenStream> {
    if extras.is_empty() {
        return Ok(TokenStream::new());
    }
    let extra = crate::path::extra_path()?;
    let service = quote!(&*__furnace_runtime.application().resolve::<#extra::RedisExtra>()?);
    let cache = extras.cache.map(|(ttl, _)| {
        quote! {
            .route_layer(#common::axum::middleware::from_fn_with_state(
                #extra::CacheState::new(#service, #ttl),
                #extra::cache_middleware,
            ))
        }
    });
    let limit = extras.rate_limit.map(|(limit, per)| {
        quote! {
            .route_layer(#common::axum::middleware::from_fn_with_state(
                #extra::RateLimitState::new(#service, #limit, #per, __furnace_routes.is_guarded(), #route),
                #extra::rate_limit_middleware,
            ))
        }
    });
    Ok(quote!(#cache #limit))
}
