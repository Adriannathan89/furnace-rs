//! Axum extraction, guard middleware, and HTTP rejection adapters.
use super::policy::authorizes;
use super::{
    Authenticated, BuiltinGuardAdapter, ErasedAuthentication, FURNACE131, GuardDescriptor,
    PassportContext, PassportError, PassportErrorKind, PassportResult, PassportStrategyAdapter,
    PassportStrategyBinding, PassportStrategyCatalog, PassportStrategyFuture, PolicyMode,
    TokenSource, VerifiedToken,
};
#[cfg(feature = "cookies")]
use crate::CookieJar;
use crate::{
    ClaimsPrincipal, Forbidden, InternalError, JwtService, JwtValidation, PassportPrincipal,
    Unauthorized,
};
use axum::{
    extract::{FromRequestParts, Request, connect_info::ConnectInfo},
    http::{HeaderValue, header::WWW_AUTHENTICATE, request::Parts},
    response::{IntoResponse, Response},
};
use furnace_rs_core::{Diagnostic, Error, Result, SourceLocation};
use std::{
    any::TypeId,
    fmt,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// Per-route middleware state produced from one preflight-selected Passport guard.
///
/// Generated route registration constructs this state while the router is built,
/// so the request path uses the exact static descriptor and selected adapter
/// already validated for that route.
#[doc(hidden)]
#[derive(Clone)]
pub struct PassportGuardState {
    application: furnace_rs_core::ApplicationContext,
    guard: &'static GuardDescriptor,
    adapter: PassportStrategyAdapter,
}

impl PassportGuardState {
    /// Constructs middleware state from an already selected static binding.
    #[doc(hidden)]
    pub fn from_binding(
        application: &furnace_rs_core::ApplicationContext,
        binding: &PassportStrategyBinding<'static>,
    ) -> Self {
        Self {
            application: application.clone(),
            guard: binding.guard(),
            adapter: binding.adapter(),
        }
    }

    /// Selects the one strategy adapter for a static guard descriptor.
    #[doc(hidden)]
    #[allow(clippy::result_large_err)]
    pub fn new(
        application: &furnace_rs_core::ApplicationContext,
        guard: &'static GuardDescriptor,
    ) -> Result<Self> {
        let guards = [guard];
        let preflight = PassportStrategyCatalog::preflight(&guards)?;
        let binding = preflight
            .binding_for(guard, None)
            .expect("the preflight result must retain its requested guard");
        Ok(Self {
            application: application.clone(),
            guard,
            adapter: binding.adapter(),
        })
    }
}

/// A route-specific Tower layer that executes a selected Passport guard.
#[doc(hidden)]
#[derive(Clone)]
pub struct PassportGuardLayer {
    state: PassportGuardState,
}

impl PassportGuardLayer {
    /// Creates one route-specific Passport middleware layer.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(state: PassportGuardState) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for PassportGuardLayer {
    type Service = PassportGuardService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        PassportGuardService {
            inner,
            state: self.state.clone(),
        }
    }
}

/// A Tower service produced by [`PassportGuardLayer`].
#[doc(hidden)]
#[derive(Clone)]
pub struct PassportGuardService<S> {
    inner: S,
    state: PassportGuardState,
}

impl<S> Service<Request> for PassportGuardService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = std::result::Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<std::result::Result<(), S::Error>> {
        self.inner.poll_ready(context)
    }

    fn call(&mut self, mut request: Request) -> Self::Future {
        let state = self.state.clone();
        let not_ready_inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, not_ready_inner);
        Box::pin(async move {
            if let Err(error) = state.authorize(&mut request).await {
                return Ok(super::PassportRejection::from(error).into_response());
            }
            inner.call(request).await
        })
    }
}

impl PassportGuardState {
    async fn authorize(&self, request: &mut Request) -> PassportResult<()> {
        let (headers, method, uri, remote_addr) = request_metadata(request);
        let authentication = authenticate_request(
            &self.application,
            self.guard.source(),
            self.adapter,
            headers,
            method,
            uri,
            remote_addr,
        )
        .await?;
        if !authorizes(self.guard, &authentication) {
            return Err(PassportError::forbidden());
        }
        if !authentication.install_extensions(request.extensions_mut()) {
            return Err(PassportError::internal(std::io::Error::other(
                "guard authentication type binding failed",
            )));
        }
        Ok(())
    }
}

/// A typed Tower layer for protecting a native Axum route with a Passport policy.
///
/// Construct it with [`PassportGuard::builder`] for a managed strategy, or
/// [`PassportGuard::<ClaimsPrincipal<C>>::jwt`] for the built-in typed-claims
/// strategy. Native guards use the same extraction, JWT verification, strategy,
/// policy, extension, and rejection pipeline as generated FURNACE routes.
///
/// ```no_run
/// use furnace_rs_common::{
///     ClaimsPrincipal, PassportGuard, PassportPrincipal,
///     axum::{Router, routing::get},
///     core::Furnace,
/// };
///
/// #[derive(serde::Deserialize)]
/// struct UserClaims { role: String }
/// impl PassportPrincipal for UserClaims {
///     fn has_role(&self, role: &str) -> bool { self.role == role }
///     fn has_permission(&self, _: &str) -> bool { false }
/// }
///
/// # async fn example(application: Furnace) -> furnace_rs_core::Result<()> {
/// let guard = PassportGuard::<ClaimsPrincipal<UserClaims>>::jwt(
///     application.context().clone(),
/// )
/// .roles_any(["user"])
/// .build()?;
/// let router: Router = Router::new()
///     .route("/profile", get(|| async { "ok" }))
///     .route_layer(guard);
/// # let _ = router;
/// # Ok(())
/// # }
/// ```
///
/// Native guards are runtime escape hatches: they are absent from the static
/// route catalog and cannot activate JWT auto-configuration. Their completed
/// application context must already contain [`JwtService`], or `build` returns
/// [`FURNACE131`].
pub struct PassportGuard<P> {
    state: NativePassportGuardState<P>,
}

impl<P> PassportGuard<P>
where
    P: PassportPrincipal,
{
    /// Starts a native Passport guard builder for the supplied application context.
    #[must_use]
    pub fn builder(application: furnace_rs_core::ApplicationContext) -> PassportGuardBuilder<P> {
        PassportGuardBuilder::new(application)
    }
}

impl<C> PassportGuard<ClaimsPrincipal<C>>
where
    C: PassportPrincipal + serde::de::DeserializeOwned,
{
    /// Starts a native guard builder using the built-in `jwt` claims strategy.
    #[must_use]
    pub fn jwt(
        application: furnace_rs_core::ApplicationContext,
    ) -> PassportGuardBuilder<ClaimsPrincipal<C>> {
        PassportGuardBuilder::new(application)
            .strategy("jwt")
            .with_builtin_adapter(builtin_claims_adapter::<C>)
    }
}

impl<P> Clone for PassportGuard<P> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<P> fmt::Debug for PassportGuard<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PassportGuard")
            .field("source", &self.state.source)
            .field("has_roles", &self.state.roles.is_some())
            .field("has_permissions", &self.state.permissions.is_some())
            .field("predicate_count", &self.state.predicates.len())
            .finish_non_exhaustive()
    }
}

/// Configures one typed native [`PassportGuard`] before it is applied to a route.
pub struct PassportGuardBuilder<P> {
    application: furnace_rs_core::ApplicationContext,
    strategy: Option<&'static str>,
    source: TokenSource,
    roles: Option<NativePolicyClause>,
    permissions: Option<NativePolicyClause>,
    predicates: Vec<fn(&P) -> bool>,
    builtin_adapter: Option<BuiltinGuardAdapter>,
}

impl<P> PassportGuardBuilder<P>
where
    P: PassportPrincipal,
{
    fn new(application: furnace_rs_core::ApplicationContext) -> Self {
        Self {
            application,
            strategy: None,
            source: TokenSource::Bearer,
            roles: None,
            permissions: None,
            predicates: Vec::new(),
            builtin_adapter: None,
        }
    }

    /// Selects the managed Passport strategy registered under `strategy`.
    #[must_use]
    pub const fn strategy(mut self, strategy: &'static str) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Selects the sole credential source accepted by this guard.
    #[must_use]
    pub const fn source(mut self, source: TokenSource) -> Self {
        self.source = source;
        self
    }

    /// Requires at least one listed principal role.
    #[must_use]
    pub fn roles_any<I, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        self.roles = Some(NativePolicyClause::new(PolicyMode::Any, values));
        self
    }

    /// Requires every listed principal role.
    #[must_use]
    pub fn roles_all<I, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        self.roles = Some(NativePolicyClause::new(PolicyMode::All, values));
        self
    }

    /// Requires at least one listed principal permission.
    #[must_use]
    pub fn permissions_any<I, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        self.permissions = Some(NativePolicyClause::new(PolicyMode::Any, values));
        self
    }

    /// Requires every listed principal permission.
    #[must_use]
    pub fn permissions_all<I, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        self.permissions = Some(NativePolicyClause::new(PolicyMode::All, values));
        self
    }

    /// Adds one synchronous principal predicate that must return `true`.
    #[must_use]
    pub fn predicate(mut self, predicate: fn(&P) -> bool) -> Self {
        self.predicates.push(predicate);
        self
    }

    fn with_builtin_adapter(mut self, adapter: BuiltinGuardAdapter) -> Self {
        self.builtin_adapter = Some(adapter);
        self
    }

    /// Validates the selected strategy and produces a reusable Tower layer.
    ///
    /// The application must already contain a concrete [`JwtService`]. Native
    /// guards do not appear in the static FURNACE route catalog, so they cannot
    /// activate JWT auto-configuration by themselves. Require `JwtService`
    /// from a managed provider before building the application, or provide a
    /// concrete service through [`furnace_rs_core::FurnaceBuilder`] before calling `build`.
    #[allow(clippy::result_large_err)]
    pub fn build(self) -> Result<PassportGuard<P>> {
        let strategy = self.strategy.ok_or_else(|| {
            native_guard_error("native Passport guards require a strategy selected with `strategy`")
        })?;
        validate_native_policy(self.roles.as_ref(), "roles")?;
        validate_native_policy(self.permissions.as_ref(), "permissions")?;

        let descriptor = GuardDescriptor::new(
            "native Axum",
            "PassportGuard",
            strategy,
            Some(native_principal_type_id::<P>),
            Some(native_principal_type_name::<P>),
            self.source,
            None,
            None,
            &[],
            SourceLocation::new("native Axum PassportGuard", 1, 1),
            self.builtin_adapter,
        );
        let guards = [&descriptor];
        let preflight = PassportStrategyCatalog::preflight(&guards)?;
        let binding = preflight
            .binding_for(&descriptor, None)
            .expect("native guard preflight must retain the requested descriptor");
        self.application.resolve::<JwtService>().map_err(|_| {
            native_guard_error("native Passport guards require an available JwtService")
        })?;

        Ok(PassportGuard {
            state: NativePassportGuardState {
                application: self.application,
                source: self.source,
                adapter: binding.adapter(),
                roles: self.roles,
                permissions: self.permissions,
                predicates: self.predicates,
                marker: PhantomData,
            },
        })
    }
}

/// A Tower service created by a native [`PassportGuard`].
#[doc(hidden)]
pub struct NativePassportGuardService<S, P> {
    inner: S,
    state: NativePassportGuardState<P>,
}

impl<S, P> Clone for NativePassportGuardService<S, P>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: self.state.clone(),
        }
    }
}

impl<P, S> Layer<S> for PassportGuard<P>
where
    P: PassportPrincipal,
{
    type Service = NativePassportGuardService<S, P>;

    fn layer(&self, inner: S) -> Self::Service {
        NativePassportGuardService {
            inner,
            state: self.state.clone(),
        }
    }
}

impl<P, S> Service<Request> for NativePassportGuardService<S, P>
where
    P: PassportPrincipal,
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = std::result::Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<std::result::Result<(), S::Error>> {
        self.inner.poll_ready(context)
    }

    fn call(&mut self, mut request: Request) -> Self::Future {
        let state = self.state.clone();
        let not_ready_inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, not_ready_inner);
        Box::pin(async move {
            if let Err(error) = state.authorize(&mut request).await {
                return Ok(super::PassportRejection::from(error).into_response());
            }
            inner.call(request).await
        })
    }
}

struct NativePassportGuardState<P> {
    application: furnace_rs_core::ApplicationContext,
    source: TokenSource,
    adapter: PassportStrategyAdapter,
    roles: Option<NativePolicyClause>,
    permissions: Option<NativePolicyClause>,
    predicates: Vec<fn(&P) -> bool>,
    marker: PhantomData<fn() -> P>,
}

impl<P> Clone for NativePassportGuardState<P> {
    fn clone(&self) -> Self {
        Self {
            application: self.application.clone(),
            source: self.source,
            adapter: self.adapter,
            roles: self.roles.clone(),
            permissions: self.permissions.clone(),
            predicates: self.predicates.clone(),
            marker: PhantomData,
        }
    }
}

impl<P> NativePassportGuardState<P>
where
    P: PassportPrincipal,
{
    async fn authorize(&self, request: &mut Request) -> PassportResult<()> {
        let (headers, method, uri, remote_addr) = request_metadata(request);
        let authentication = authenticate_request(
            &self.application,
            self.source,
            self.adapter,
            headers,
            method,
            uri,
            remote_addr,
        )
        .await?;
        let Some(principal) = authentication.principal_as::<P>() else {
            return Err(PassportError::internal(std::io::Error::other(
                "native guard authentication type binding failed",
            )));
        };
        if !matches_native_clause(self.roles.as_ref(), |value| principal.has_role(value))
            || !matches_native_clause(self.permissions.as_ref(), |value| {
                principal.has_permission(value)
            })
            || !self
                .predicates
                .iter()
                .all(|predicate| predicate(&principal))
        {
            return Err(PassportError::forbidden());
        }
        if !authentication.install_extensions(request.extensions_mut()) {
            return Err(PassportError::internal(std::io::Error::other(
                "native guard authentication type binding failed",
            )));
        }
        Ok(())
    }
}

#[derive(Clone)]
struct NativePolicyClause {
    mode: PolicyMode,
    values: Vec<String>,
}

impl NativePolicyClause {
    fn new<I, V>(mode: PolicyMode, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        Self {
            mode,
            values: values
                .into_iter()
                .map(|value| value.as_ref().to_owned())
                .collect(),
        }
    }
}

async fn authenticate_request(
    application: &furnace_rs_core::ApplicationContext,
    source: TokenSource,
    adapter: PassportStrategyAdapter,
    headers: axum::http::HeaderMap,
    method: axum::http::Method,
    uri: axum::http::Uri,
    remote_addr: Option<std::net::SocketAddr>,
) -> PassportResult<ErasedAuthentication> {
    match source {
        TokenSource::Bearer => {
            super::authentication::authenticate_bearer(
                application,
                adapter,
                &headers,
                &method,
                &uri,
                remote_addr,
            )
            .await
        }
        #[cfg(feature = "cookies")]
        TokenSource::Cookie(name) => {
            let jar = CookieJar::from_headers(&headers).map_err(|_| PassportError::reject())?;
            if jar.occurrences(name) != 1 {
                return Err(PassportError::reject());
            }
            let token = jar
                .get(name)
                .map(|cookie| cookie.value().to_owned())
                .ok_or_else(PassportError::reject)?;
            let context = PassportContext::with_cookie_token(
                &headers,
                &method,
                &uri,
                remote_addr,
                &jar,
                name,
            );
            adapter(application, &context, &token).await
        }
    }
}

fn request_metadata(
    request: &Request,
) -> (
    axum::http::HeaderMap,
    axum::http::Method,
    axum::http::Uri,
    Option<std::net::SocketAddr>,
) {
    (
        request.headers().clone(),
        request.method().clone(),
        request.uri().clone(),
        request
            .extensions()
            .get::<ConnectInfo<std::net::SocketAddr>>()
            .map(|address| address.0),
    )
}

fn matches_native_clause(
    clause: Option<&NativePolicyClause>,
    matches: impl FnMut(&str) -> bool,
) -> bool {
    let Some(clause) = clause else {
        return true;
    };
    match clause.mode {
        PolicyMode::Any => clause.values.iter().map(String::as_str).any(matches),
        PolicyMode::All => clause.values.iter().map(String::as_str).all(matches),
    }
}

fn validate_native_policy(clause: Option<&NativePolicyClause>, label: &str) -> Result<()> {
    let Some(clause) = clause else {
        return Ok(());
    };
    if clause.values.is_empty()
        || clause
            .values
            .iter()
            .any(|value| value.is_empty() || value.chars().any(char::is_control))
    {
        return Err(native_guard_error(format!(
            "native guard {label} policy must contain non-empty values without control characters"
        )));
    }
    Ok(())
}

fn native_principal_type_id<P>() -> TypeId
where
    P: PassportPrincipal,
{
    TypeId::of::<P>()
}

fn native_principal_type_name<P>() -> &'static str
where
    P: PassportPrincipal,
{
    std::any::type_name::<P>()
}

fn native_guard_error(message: impl Into<String>) -> Error {
    Error::new(
        Diagnostic::new(FURNACE131, "invalid native Passport guard", message)
            .with_subject("native Axum PassportGuard")
            .with_location(SourceLocation::new("native Axum PassportGuard", 1, 1)),
    )
}

fn builtin_claims_adapter<'a, C>(
    application: &'a furnace_rs_core::ApplicationContext,
    _context: &'a PassportContext<'a>,
    token: &'a str,
) -> PassportStrategyFuture<'a>
where
    C: PassportPrincipal + serde::de::DeserializeOwned,
{
    Box::pin(async move {
        let jwt = application
            .resolve::<JwtService>()
            .map_err(PassportError::internal)?;
        let verified = std::sync::Arc::new(
            jwt.verify::<C>(token, JwtValidation::access())
                .map_err(PassportError::from)?,
        );
        let principal = ClaimsPrincipal::<C>::new(std::sync::Arc::clone(&verified));
        Ok(ErasedAuthentication::with_verified(principal, verified))
    })
}

impl<P, S> FromRequestParts<S> for Authenticated<P>
where
    P: PassportPrincipal,
    S: Send + Sync,
{
    type Rejection = PassportRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or_else(|| {
            PassportError::internal(MissingGuardExtension("authenticated principal")).into()
        })
    }
}

impl<C, S> FromRequestParts<S> for VerifiedToken<C>
where
    C: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = PassportRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or_else(|| PassportError::internal(MissingGuardExtension("verified token")).into())
    }
}

#[derive(Debug)]
struct MissingGuardExtension(&'static str);

impl fmt::Display for MissingGuardExtension {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "missing guard-installed {}", self.0)
    }
}

impl std::error::Error for MissingGuardExtension {}
/// An Axum rejection produced by Passport extractors and guards.
#[non_exhaustive]
pub struct PassportRejection(PassportError);

impl PassportRejection {
    pub(crate) const fn new(error: PassportError) -> Self {
        Self(error)
    }

    /// Returns the stable category of the rejected Passport operation.
    #[must_use]
    pub const fn kind(&self) -> PassportErrorKind {
        self.0.kind()
    }
}

impl fmt::Debug for PassportRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PassportRejection")
            .field("kind", &self.kind())
            .finish()
    }
}

impl fmt::Display for PassportRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for PassportRejection {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<PassportError> for PassportRejection {
    fn from(error: PassportError) -> Self {
        Self::new(error)
    }
}

impl IntoResponse for PassportRejection {
    fn into_response(self) -> Response {
        let Self(error) = self;
        match error.kind() {
            PassportErrorKind::Rejected => {
                let mut response = Unauthorized::new("authentication was rejected").into_response();
                response
                    .headers_mut()
                    .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
                response
            }
            PassportErrorKind::Forbidden => Forbidden::new("access was denied").into_response(),
            PassportErrorKind::Internal => InternalError::new(error).into_response(),
        }
    }
}

impl<'a> super::PassportStrategyPreflight<'a> {
    pub(crate) fn binding_for_endpoint(
        &self,
        occurrence: crate::http_scope::RouteIdentity,
    ) -> Option<&super::PassportStrategyBinding<'a>> {
        self.binding_for_occurrence(
            &occurrence.guard_occurrence(),
            occurrence.passport_context_cauldron(),
        )
    }
}
