//! Shared endpoint syntax, native extractor validation, and handler adapters.
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::{
    Attribute, Error, FnArg, LitStr, Meta, Token, Type, punctuated::Punctuated, spanned::Spanned,
};
const VERBS: &[&str] = &["get", "post", "put", "patch", "delete"];
/// Identifies body-consuming extractors whose crate path is known to FURNACE.
///
/// This intentionally inspects syntax only. A bare `Json` or an application
/// extractor with a different path may resolve to any type, so its body
/// behavior remains the native Axum/rustc contract.
pub(crate) fn validate_body_extractor_order(
    inputs: &Punctuated<FnArg, Token![,]>,
    common: &syn::Path,
) -> syn::Result<()> {
    let skip = usize::from(matches!(inputs.first(), Some(FnArg::Receiver(_))));
    let arguments = inputs.iter().skip(skip).collect::<Vec<_>>();
    for (index, argument) in arguments.iter().enumerate() {
        let FnArg::Typed(argument) = argument else {
            continue;
        };
        if known_body_consumer(&argument.ty, common) && index + 1 != arguments.len() {
            return Err(Error::new(
                argument.ty.span(),
                "known body extractors (`Json`, `ValidatedJson`, and `Request`) must be the final route parameter",
            ));
        }
    }
    Ok(())
}

pub(crate) fn known_body_consumer(ty: &Type, common: &syn::Path) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };
    if type_path.qself.is_some() {
        return false;
    }

    let path = &type_path.path;
    if is_furnace_type(path, common, "Json") || is_axum_type(path, common, "Json") {
        return true;
    }
    if is_furnace_type(path, common, "ValidatedJson") {
        return true;
    }
    if is_furnace_type(path, common, "Request") || is_axum_type(path, common, "Request") {
        return true;
    }
    false
}

fn is_furnace_type(path: &syn::Path, common: &syn::Path, name: &str) -> bool {
    path_with_suffix_is(path, common, &[name]) || facade_path_with_suffix_is(path, common, &[name])
}

fn is_axum_type(path: &syn::Path, common: &syn::Path, name: &str) -> bool {
    path_is(path, &["axum", name])
        || path_is(path, &["axum", "extract", name])
        || path_with_suffix_is(path, common, &["axum", name])
        || path_with_suffix_is(path, common, &["axum", "extract", name])
        || facade_path_with_suffix_is(path, common, &["axum", name])
        || facade_path_with_suffix_is(path, common, &["axum", "extract", name])
}

fn facade_path_with_suffix_is(path: &syn::Path, common: &syn::Path, suffix: &[&str]) -> bool {
    let Some(last) = common.segments.last() else {
        return false;
    };
    if last.ident != "common" {
        return false;
    }

    let prefix_len = common.segments.len() - 1;
    let path_len = path.segments.len();
    path_len == prefix_len + suffix.len()
        && path
            .segments
            .iter()
            .take(prefix_len)
            .zip(common.segments.iter().take(prefix_len))
            .all(|(actual, expected)| actual.ident == expected.ident)
        && path
            .segments
            .iter()
            .skip(prefix_len)
            .zip(suffix)
            .all(|(segment, expected)| segment.ident == *expected)
}

fn path_with_suffix_is(path: &syn::Path, prefix: &syn::Path, suffix: &[&str]) -> bool {
    let prefix_len = prefix.segments.len();
    let path_len = path.segments.len();
    path_len == prefix_len + suffix.len()
        && path
            .segments
            .iter()
            .take(prefix_len)
            .zip(prefix.segments.iter())
            .all(|(actual, expected)| actual.ident == expected.ident)
        && path
            .segments
            .iter()
            .skip(prefix_len)
            .zip(suffix)
            .all(|(segment, expected)| segment.ident == *expected)
}

fn path_is(path: &syn::Path, expected: &[&str]) -> bool {
    path.segments.len() == expected.len()
        && path
            .segments
            .iter()
            .zip(expected)
            .all(|(segment, expected)| segment.ident == *expected)
}

pub(crate) fn is_conditional_attribute(attribute: &Attribute) -> bool {
    attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
}

#[derive(Clone, Copy)]
pub(crate) enum HttpVerb {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpVerb {
    pub(crate) fn from_name(name: &str) -> Self {
        match name {
            "get" => Self::Get,
            "post" => Self::Post,
            "put" => Self::Put,
            "patch" => Self::Patch,
            "delete" => Self::Delete,
            _ => unreachable!("validated route verbs are exhaustive"),
        }
    }

    pub(crate) fn tokens(self, common: &syn::Path) -> proc_macro2::TokenStream {
        match self {
            Self::Get => quote!(#common::HttpMethod::Get),
            Self::Post => quote!(#common::HttpMethod::Post),
            Self::Put => quote!(#common::HttpMethod::Put),
            Self::Patch => quote!(#common::HttpMethod::Patch),
            Self::Delete => quote!(#common::HttpMethod::Delete),
        }
    }

    pub(crate) fn routing_tokens(self, common: &syn::Path) -> proc_macro2::TokenStream {
        match self {
            Self::Get => quote!(#common::__private::get),
            Self::Post => quote!(#common::__private::post),
            Self::Put => quote!(#common::__private::put),
            Self::Patch => quote!(#common::__private::patch),
            Self::Delete => quote!(#common::__private::delete),
        }
    }
}

pub(crate) fn route_verb(attribute: &Attribute) -> Option<&'static str> {
    route_verb_path(attribute.path())
}

fn route_verb_path(path: &syn::Path) -> Option<&'static str> {
    let ident = path.segments.last()?.ident.to_string();
    VERBS.iter().copied().find(|verb| ident == *verb)
}

pub(crate) fn cfg_attr_contains_route_verb(attribute: &Attribute) -> syn::Result<bool> {
    let nested = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    Ok(nested
        .iter()
        .skip(1)
        .any(|meta| route_verb_path(meta.path()).is_some()))
}

pub(crate) fn join_paths(prefix: &LitStr, path: &LitStr) -> syn::Result<LitStr> {
    let prefix = prefix.value();
    let path_value = path.value();
    let full_path = if prefix.is_empty() || prefix == "/" {
        path_value
    } else if path_value == "/" {
        prefix
    } else {
        format!("{prefix}{path_value}")
    };
    Ok(LitStr::new(&full_path, path.span()))
}

/// Names used to pass Axum-extracted arguments into an authored method.
pub(crate) fn argument_names(types: &[Type]) -> Vec<syn::Ident> {
    types
        .iter()
        .enumerate()
        .map(|(index, _)| format_ident!("__furnace_argument_{index}"))
        .collect()
}

/// Typed adapter inputs shared by legacy and inherent endpoint expanders.
pub(crate) struct HandlerAdapter<'a> {
    pub(crate) verb: HttpVerb,
    pub(crate) handler: &'a LitStr,
    pub(crate) argument_types: &'a [Type],
    pub(crate) conditional_attributes: &'a [Attribute],
    pub(crate) invocation: TokenStream,
    pub(crate) guard_layer: TokenStream,
}

impl HandlerAdapter<'_> {
    pub(crate) fn tokens(self, common: &syn::Path) -> TokenStream {
        let method = self.verb.tokens(common);
        let routing = self.verb.routing_tokens(common);
        let handler = self.handler;
        let types = self.argument_types;
        let args = argument_names(types);
        let attrs = self.conditional_attributes;
        let invocation = self.invocation;
        let guard_layer = self.guard_layer;
        quote! {
            #(#attrs)*
            {
                if let Some(__furnace_path) = __furnace_routes.next(#method, #handler)? {
                    let __furnace_handler_controller = __furnace_controller.clone();
                    __furnace_router = __furnace_router.route(
                        __furnace_path,
                        #routing(move |#(#args: #types),*| {
                            let __furnace_controller = __furnace_handler_controller.clone();
                            async move { #invocation }
                        }) #guard_layer,
                    );
                }
            }
        }
    }
}

/// Normalizes colon captures and validates native brace and wildcard paths.
pub(crate) fn canonical_path(path: &LitStr, is_prefix: bool) -> syn::Result<LitStr> {
    let input = path.value();
    let value = if is_prefix && input.len() > 1 {
        input.trim_end_matches('/')
    } else {
        &input
    };
    let error = |message| Error::new(path.span(), message);
    if is_prefix && value.is_empty() {
        return Ok(LitStr::new("", path.span()));
    }
    if value.is_empty() || !value.starts_with('/') {
        return Err(error("endpoint paths must start with `/`"));
    }
    if value.contains(['?', '#', '\\', '%'])
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(error(
            "endpoint paths must not contain queries, fragments, escapes, control characters, or whitespace",
        ));
    }
    if value == "/" {
        return Ok(LitStr::new(value, path.span()));
    }
    let segments: Vec<_> = value.split('/').skip(1).collect();
    let mut names = BTreeSet::new();
    let mut normalized = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        if segment.is_empty() || matches!(*segment, "." | "..") {
            return Err(error(
                "endpoint paths must not contain empty, `.` or `..` segments",
            ));
        }
        let capture = if let Some(name) = segment.strip_prefix(':') {
            Some((name, false))
        } else if let Some(name) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            Some(
                name.strip_prefix('*')
                    .map_or((name, false), |name| (name, true)),
            )
        } else {
            segment.strip_prefix('*').map(|name| (name, true))
        };
        if let Some((name, wildcard)) = capture {
            if is_prefix {
                return Err(error("controller route prefixes must not contain captures"));
            }
            let mut chars = name.chars();
            if !chars
                .next()
                .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
                || !chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
            {
                return Err(error("capture names must use `[A-Za-z_][A-Za-z0-9_]*`"));
            }
            if !names.insert(name) {
                return Err(error("endpoint paths must not repeat capture names"));
            }
            if wildcard && index + 1 != segments.len() {
                return Err(error("wildcard captures must be the final path segment"));
            }
            normalized.push(if wildcard {
                format!("{{*{name}}}")
            } else {
                format!("{{{name}}}")
            });
        } else {
            if segment.contains([':', '{', '}', '*']) {
                return Err(error("captures must occupy an entire path segment"));
            }
            normalized.push((*segment).to_owned());
        }
    }
    Ok(LitStr::new(
        &format!("/{}", normalized.join("/")),
        path.span(),
    ))
}
