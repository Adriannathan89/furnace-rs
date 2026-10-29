//! Credential selection and shared Passport admission before upgrade.
use crate::{SocketEndpointDescriptor, SocketHandshake, runtime::SocketRuntime};
use furnace_rs_core::ApplicationContext;
use http::StatusCode;
use http::request::Parts;
use std::{net::SocketAddr, sync::Arc};

pub(crate) async fn admit(
    app: ApplicationContext,
    endpoint: &SocketEndpointDescriptor,
    request: &Parts,
    runtime: &SocketRuntime,
) -> Result<SocketHandshake, StatusCode> {
    let origins = request
        .headers
        .get_all(http::header::ORIGIN)
        .iter()
        .collect::<Vec<_>>();

    if origins.len() > 1
        || origins.first().is_some_and(|v| {
            v.to_str()
                .ok()
                .is_none_or(|v| !runtime.config.origins.iter().any(|o| o == v))
        })
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let peer = request.extensions.get::<SocketAddr>().copied();
    #[allow(unused_mut)]
    let mut extensions = http::Extensions::new();

    if endpoint.guard.is_some() {
        #[cfg(feature = "jwt")]
        {
            use furnace_rs_common::passport::__private::{authenticate_bearer, authorizes};
            let binding = runtime
                .bindings
                .get(endpoint.path)
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
            let token = credential(
                &request.headers,
                &request.uri,
                endpoint.cookie,
                endpoint.query,
            )?;
            let mut headers = request.headers.clone();
            headers.remove(http::header::AUTHORIZATION);
            headers.remove(http::header::COOKIE);
            headers.insert(
                http::header::AUTHORIZATION,
                http::HeaderValue::from_str(&format!("Bearer {token}"))
                    .map_err(|_| StatusCode::UNAUTHORIZED)?,
            );
            let uri = request
                .uri
                .path()
                .parse::<http::Uri>()
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let auth = authenticate_bearer(
                &app,
                binding.adapter(),
                &headers,
                &request.method,
                &uri,
                peer,
            )
            .await
            .map_err(|e| match e.kind() {
                furnace_rs_common::PassportErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::UNAUTHORIZED,
            })?;

            if !authorizes(binding.guard(), &auth) {
                return Err(StatusCode::FORBIDDEN);
            }

            if !auth.install_extensions(&mut extensions) {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }

        #[cfg(not(feature = "jwt"))]
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(SocketHandshake {
        application: app,
        headers: request.headers.clone(),
        method: request.method.clone(),
        uri: request.uri.clone(),
        peer,
        extensions: Arc::new(extensions),
    })
}

#[cfg(feature = "jwt")]
fn credential(
    headers: &http::HeaderMap,
    uri: &http::Uri,
    cookie_name: &str,
    query_name: &str,
) -> Result<String, StatusCode> {
    let mut tokens = Vec::new();
    let auth = headers
        .get_all(http::header::AUTHORIZATION)
        .iter()
        .collect::<Vec<_>>();

    if auth.len() > 1 {
        return Err(StatusCode::BAD_REQUEST);
    }

    if let Some(auth) = auth.first() {
        let auth = auth.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?.trim();
        let (space, token) = auth.split_once(' ').ok_or(StatusCode::UNAUTHORIZED)?;

        if !space.eq_ignore_ascii_case("bearer") || token.trim().is_empty() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        tokens.push(token.trim().to_owned());
    }

    for header in headers.get_all(http::header::COOKIE) {
        let value = header.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;

        for item in cookie::Cookie::split_parse(value) {
            let cookie = item.map_err(|_| StatusCode::BAD_REQUEST)?;

            if cookie.name() == cookie_name {
                tokens.push(cookie.value().to_owned());
            }
        }
    }

    let params: Vec<(String, String)> = serde_urlencoded::from_str(uri.query().unwrap_or(""))
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    tokens.extend(
        params
            .into_iter()
            .filter(|(key, _)| key == query_name)
            .map(|(_, value)| value),
    );

    if tokens.len() != 1 {
        return Err(if tokens.is_empty() {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::BAD_REQUEST
        });
    }

    Ok(tokens.pop().unwrap())
}

#[cfg(feature = "jwt")]
#[allow(clippy::result_large_err)]
pub(crate) fn guards(
    graph: Option<&furnace_rs_core::CauldronGraph>,
) -> furnace_rs_core::Result<Vec<furnace_rs_common::passport::__private::ScopedGuard>> {
    use furnace_rs_common::passport::__private::{GuardOccurrence, ScopedGuard};

    Ok(crate::SocketApplicationScope::for_cauldron_graph(graph)?
        .endpoints()
        .iter()
        .filter_map(|endpoint| {
            endpoint.guard.map(|guard| ScopedGuard {
                guard: (guard.descriptor)(),
                occurrence: Some(GuardOccurrence {
                    provider: (endpoint.provider)(),
                    operation: endpoint.path.to_owned(),
                }),
                context_cauldron: graph
                    .and_then(|g| g.owner_of((endpoint.provider)()))
                    .map(|n| n.type_id()),
            })
        })
        .collect())
}

#[cfg(feature = "jwt")]
fn passport_scope(
    context: &furnace_rs_core::__private::AutoConfigurationContext<'_>,
) -> furnace_rs_core::Result<Vec<furnace_rs_common::passport::__private::ScopedGuard>> {
    guards(context.cauldron_graph())
}

#[cfg(feature = "jwt")]
furnace_rs_core::__private::inventory::submit! {furnace_rs_common::passport::__private::PassportScopeDescriptor{select:passport_scope}}
