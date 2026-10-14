//! Managed, cloneable outbound channels with local credential preparation.
use super::{FURNACE153, GrpcClientConfig, GrpcClientError, GrpcClientResult, GrpcConnectionMode};
use furnace_rs_core::{
    ApplicationContext, Diagnostic, Error, LifecycleFuture, LifecycleHook, Secret,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use tonic::{body::Body, transport::Channel};
use tower::{Service, ServiceExt};
/// Supplies a fresh credential asynchronously for each outbound RPC.
pub trait GrpcTokenProvider: Clone + Send + Sync + 'static {
    /// Retrieves a token; failures stop the call before remote dispatch.
    fn token(&self) -> impl Future<Output = GrpcClientResult<Secret<String>>> + Send;
}
type TokenFuture = Pin<Box<dyn Future<Output = GrpcClientResult<Secret<String>>> + Send>>;
/// Type-erased DI credential provider used by generated wrappers.
#[doc(hidden)]
#[derive(Clone)]
pub struct ErasedTokenProvider(Arc<dyn Fn() -> TokenFuture + Send + Sync>);
impl ErasedTokenProvider {
    /// Captures a typed provider without requesting a credential.
    pub fn of<P: GrpcTokenProvider>(provider: P) -> Self {
        Self(Arc::new(move || {
            let provider = provider.clone();
            Box::pin(async move { provider.token().await })
        }))
    }
}
/// Validates and replaces explicit authorization metadata on one native request.
pub fn set_grpc_bearer_token<T>(
    request: &mut tonic::Request<T>,
    token: &str,
) -> GrpcClientResult<()> {
    let value = bearer_value(token)?;
    request.metadata_mut().remove("authorization");
    request.metadata_mut().insert("authorization", value);
    Ok(())
}
fn bearer_value(
    token: &str,
) -> GrpcClientResult<tonic::metadata::MetadataValue<tonic::metadata::Ascii>> {
    let value = http::HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| GrpcClientError::invalid_credential())?;
    let mut headers = http::HeaderMap::new();
    headers.insert(http::header::AUTHORIZATION, value);
    if crate::bearer::parse(&headers) != Some(token) {
        return Err(GrpcClientError::invalid_credential());
    }
    format!("Bearer {token}")
        .parse()
        .map_err(|_| GrpcClientError::invalid_credential())
}
struct State {
    config: GrpcClientConfig,
    provider: Option<ErasedTokenProvider>,
    channel: Mutex<Option<Channel>>,
    closed: AtomicBool,
}
/// Cheaply cloned managed transport accepted by generated Tonic clients.
#[derive(Clone)]
pub struct GrpcChannel(Arc<State>);
impl std::fmt::Debug for GrpcChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcChannel")
            .field("closed", &self.0.closed.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}
impl GrpcChannel {
    /// Constructs shared state without DNS, connection, or credential-provider I/O.
    pub async fn from_config(
        config: GrpcClientConfig,
        provider: Option<ErasedTokenProvider>,
    ) -> furnace_rs_core::Result<Self> {
        Ok(Self(Arc::new(State {
            config,
            provider,
            channel: Mutex::new(None),
            closed: AtomicBool::new(false),
        })))
    }
    /// Establishes initial connection readiness within the configured connect deadline.
    pub async fn ready(&self) -> furnace_rs_core::Result<()> {
        if self.0.closed.load(Ordering::SeqCst) {
            return Err(readiness_error());
        }
        let endpoint = self.endpoint().await.map_err(|_| readiness_error())?;
        let channel = tokio::time::timeout(self.0.config.connect_timeout(), endpoint.connect())
            .await
            .map_err(|_| readiness_error())?
            .map_err(|_| readiness_error())?;
        let mut stored = self.0.channel.lock().expect("channel lock poisoned");
        if self.0.closed.load(Ordering::SeqCst) {
            return Err(readiness_error());
        }
        *stored = Some(channel);
        Ok(())
    }
    /// Stops new calls and releases the channel held by shared state.
    pub fn close(&self) {
        self.0.closed.store(true, Ordering::SeqCst);
        self.0.channel.lock().expect("channel lock poisoned").take();
    }
    async fn endpoint(&self) -> GrpcClientResult<tonic::transport::Endpoint> {
        Ok(self
            .0
            .config
            .endpoint()
            .connect_timeout(self.0.config.connect_timeout()))
    }
    async fn channel(&self) -> GrpcClientResult<Channel> {
        if self.0.closed.load(Ordering::SeqCst) {
            return Err(GrpcClientError::closed());
        }
        if let Some(channel) = self
            .0
            .channel
            .lock()
            .expect("channel lock poisoned")
            .as_ref()
        {
            return Ok(channel.clone());
        }
        let channel = self.endpoint().await?.connect_lazy();
        let mut stored = self.0.channel.lock().expect("channel lock poisoned");
        if self.0.closed.load(Ordering::SeqCst) {
            return Err(GrpcClientError::closed());
        }
        Ok(stored.get_or_insert(channel).clone())
    }
}
impl Service<http::Request<Body>> for GrpcChannel {
    type Response = http::Response<Body>;
    type Error = GrpcClientError;
    type Future = Pin<Box<dyn Future<Output = GrpcClientResult<Self::Response>> + Send>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<GrpcClientResult<()>> {
        if self.0.closed.load(Ordering::SeqCst) {
            Poll::Ready(Err(GrpcClientError::closed()))
        } else {
            Poll::Ready(Ok(()))
        }
    }
    fn call(&mut self, mut request: http::Request<Body>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            let deadline = request_deadline(&mut request, this.0.config.request_timeout())?;
            let dispatch = async {
                // Validate existing credentials before provider invocation or channel creation.
                if request.headers().contains_key(http::header::AUTHORIZATION) {
                    crate::bearer::parse(request.headers())
                        .ok_or_else(GrpcClientError::invalid_credential)?;
                } else if let Some(provider) = &this.0.provider {
                    let token = (provider.0)().await?;
                    let value = bearer_value(token.expose())?;
                    request.headers_mut().insert(
                        http::header::AUTHORIZATION,
                        http::HeaderValue::from_bytes(value.as_encoded_bytes())
                            .expect("validated bearer metadata"),
                    );
                }
                let mut channel = this.channel().await?;
                channel.ready().await.map_err(GrpcClientError::transport)?;
                match channel.call(request).await {
                    Ok(response) => Ok(response),
                    Err(error) if native_timeout(&error) => {
                        Ok(tonic::Status::deadline_exceeded("deadline exceeded").into_http())
                    }
                    Err(error) => Err(GrpcClientError::transport(error)),
                }
            };
            match deadline {
                Some(timeout) => match tokio::time::timeout(timeout, dispatch).await {
                    Ok(result) => result,
                    Err(_) => Ok(tonic::Status::deadline_exceeded("deadline exceeded").into_http()),
                },
                None => dispatch.await,
            }
        })
    }
}
fn request_deadline(
    request: &mut http::Request<Body>,
    default: Option<Duration>,
) -> GrpcClientResult<Option<Duration>> {
    let mut values = request.headers().get_all("grpc-timeout").iter();
    if let Some(value) = values.next() {
        if values.next().is_some() {
            return Err(GrpcClientError::transport(std::io::Error::other(
                "invalid RPC timeout",
            )));
        }
        let raw = value.to_str().map_err(GrpcClientError::transport)?;
        let duration = parse_timeout(raw).ok_or_else(|| {
            GrpcClientError::transport(std::io::Error::other("invalid RPC timeout"))
        })?;
        return Ok(Some(duration));
    }
    if let Some(timeout) = default {
        let mut native = tonic::Request::new(());
        native.set_timeout(timeout);
        let value = native
            .metadata()
            .get("grpc-timeout")
            .expect("native timeout header")
            .clone();
        request.headers_mut().insert(
            "grpc-timeout",
            http::HeaderValue::from_bytes(value.as_encoded_bytes())
                .expect("native timeout metadata"),
        );
    }
    Ok(default)
}
fn parse_timeout(raw: &str) -> Option<Duration> {
    let (number, unit) = raw.split_at_checked(raw.len().checked_sub(1)?)?;
    if number.is_empty() || number.len() > 8 || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: u64 = number.parse().ok()?;
    Some(match unit {
        "H" => Duration::from_secs(value.checked_mul(3600)?),
        "M" => Duration::from_secs(value.checked_mul(60)?),
        "S" => Duration::from_secs(value),
        "m" => Duration::from_millis(value),
        "u" => Duration::from_micros(value),
        "n" => Duration::from_nanos(value),
        _ => return None,
    })
}
fn readiness_error() -> Error {
    Error::new(Diagnostic::new(
        FURNACE153,
        "gRPC client readiness failed",
        "the named client could not establish a verified transport connection",
    ))
}
/// Infrastructure lifecycle adapter installed by generated named clients.
#[doc(hidden)]
pub struct GrpcClientLifecycle {
    channel: GrpcChannel,
    name: &'static str,
}
impl GrpcClientLifecycle {
    /// Retains the managed channel for exact-once startup and shutdown hooks.
    pub fn new(channel: GrpcChannel, name: &'static str) -> Self {
        Self { channel, name }
    }
}
impl LifecycleHook for GrpcClientLifecycle {
    fn name(&self) -> &str {
        self.name
    }
    fn start<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            if self.channel.0.config.connection_mode() == GrpcConnectionMode::Eager {
                self.channel.ready().await?;
            }
            Ok(())
        })
    }
    fn stop<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            self.channel.close();
            Ok(())
        })
    }
}

fn native_timeout(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut cause = Some(error);
    while let Some(error) = cause {
        if error.is::<tonic::TimeoutExpired>() {
            return true;
        }
        cause = error.source();
    }
    false
}
