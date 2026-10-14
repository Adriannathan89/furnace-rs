//! Asynchronous Passport admission around native generated Tonic services.
use crate::common::passport::__private::{authenticate_bearer, authorizes};
use crate::{
    Authenticated, PassportError, PassportErrorKind, PassportPrincipal, PassportStrategyBinding,
    VerifiedToken,
};
use furnace_rs_core::ApplicationContext;
use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tonic::{Status, body::Body, server::NamedService};
use tower::Service;

/// Reads the exact typed principal installed at RPC admission.
pub fn grpc_authenticated<P: PassportPrincipal, T>(
    request: &tonic::Request<T>,
) -> Result<Authenticated<P>, Status> {
    request
        .extensions()
        .get::<Authenticated<P>>()
        .cloned()
        .ok_or_else(|| Status::internal("internal server error"))
}
/// Reads the exact verified claims installed at RPC admission.
pub fn grpc_verified_token<C: Send + Sync + 'static, T>(
    request: &tonic::Request<T>,
) -> Result<VerifiedToken<C>, Status> {
    request
        .extensions()
        .get::<VerifiedToken<C>>()
        .cloned()
        .ok_or_else(|| Status::internal("internal server error"))
}
/// Maps Passport failures to fixed, credential-free gRPC statuses.
pub fn passport_status(error: PassportError) -> Status {
    match error.kind() {
        PassportErrorKind::Rejected => Status::unauthenticated("unauthenticated"),
        PassportErrorKind::Forbidden => Status::permission_denied("permission denied"),
        PassportErrorKind::Internal => Status::internal("internal server error"),
        _ => Status::internal("internal server error"),
    }
}
/// Generated-service adapter retaining the native service name and readiness contract.
#[doc(hidden)]
#[derive(Clone)]
pub struct GrpcPassportService<S> {
    inner: S,
    application: ApplicationContext,
    bindings: Arc<BTreeMap<String, Option<PassportStrategyBinding<'static>>>>,
}
impl<S> GrpcPassportService<S> {
    pub(crate) fn new(
        inner: S,
        application: ApplicationContext,
        bindings: BTreeMap<String, Option<PassportStrategyBinding<'static>>>,
    ) -> Self {
        Self {
            inner,
            application,
            bindings: Arc::new(bindings),
        }
    }
}
impl<S: NamedService> NamedService for GrpcPassportService<S> {
    const NAME: &'static str = S::NAME;
}
impl<S> Service<http::Request<Body>> for GrpcPassportService<S>
where
    S: Service<http::Request<Body>, Response = http::Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = http::Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, mut request: http::Request<Body>) -> Self::Future {
        let application = self.application.clone();
        let binding = self.bindings.get(request.uri().path()).cloned();
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            let Some(binding) = binding else {
                return Ok(Status::unimplemented("unimplemented method").into_http());
            };
            if let Some(binding) = binding {
                let peer = request
                    .extensions()
                    .get::<tonic::transport::server::TcpConnectInfo>()
                    .and_then(|info| info.remote_addr());
                #[cfg(feature = "tls")]
                let peer = peer.or_else(|| {
                    request
                        .extensions()
                        .get::<tonic::transport::server::TlsConnectInfo<
                            tonic::transport::server::TcpConnectInfo,
                        >>()
                        .and_then(|info| info.get_ref().remote_addr())
                });
                let result = authenticate_bearer(
                    &application,
                    binding.adapter(),
                    request.headers(),
                    request.method(),
                    request.uri(),
                    peer,
                )
                .await;
                let result = result.and_then(|authentication| {
                    if !authorizes(binding.guard(), &authentication) {
                        return Err(PassportError::forbidden());
                    }
                    if !authentication.install_extensions(request.extensions_mut()) {
                        return Err(PassportError::internal(std::io::Error::other(
                            "authentication type binding failed",
                        )));
                    }
                    Ok(())
                });
                if let Err(error) = result {
                    return Ok(passport_status(error).into_http());
                }
            }
            inner.call(request).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ErasedAuthentication, GuardDescriptor, JwtService, JwtSignOptions, JwtValidation,
        PassportStrategyCatalog, PassportStrategyFuture, TokenSource,
    };
    use furnace_grpc_test_support::pb;
    use furnace_rs_core::{ConfigBuilder, Furnace, MapSource, SourceLocation};
    use std::{
        any::{TypeId, type_name},
        sync::atomic::{AtomicU64, Ordering},
        time::Duration,
    };
    use tonic::codegen::tokio_stream::{self as streams, Stream};
    static NOW: AtomicU64 = AtomicU64::new(0);
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    struct Claims {
        id: u64,
    }
    impl PassportPrincipal for Claims {
        fn has_role(&self, _: &str) -> bool {
            false
        }
        fn has_permission(&self, _: &str) -> bool {
            false
        }
    }
    fn adapter<'a>(
        app: &'a ApplicationContext,
        _: &'a crate::PassportContext<'a>,
        token: &'a str,
    ) -> PassportStrategyFuture<'a> {
        Box::pin(async move {
            let jwt = app
                .resolve::<JwtService>()
                .map_err(PassportError::internal)?;
            let verified = jwt.verify::<Claims>(token, JwtValidation::access())?;
            if verified.claims.registered.expires_at <= NOW.load(Ordering::SeqCst) {
                return Err(PassportError::reject());
            }
            Ok(ErasedAuthentication::new(
                verified.claims.custom.clone(),
                verified,
            ))
        })
    }
    static GUARD: GuardDescriptor = GuardDescriptor::new(
        "Expiry",
        "seal",
        "jwt",
        Some(TypeId::of::<Claims>),
        Some(type_name::<Claims>),
        TokenSource::Bearer,
        None,
        None,
        &[],
        SourceLocation::new(file!(), line!(), column!()),
        Some(adapter),
    );
    type Output = Pin<Box<dyn Stream<Item = Result<pb::Item, Status>> + Send>>;
    type Input = tokio::sync::mpsc::Receiver<Result<pb::Item, Status>>;
    #[derive(Clone)]
    struct ExpiryService {
        input: Arc<tokio::sync::Mutex<Option<Input>>>,
    }
    #[tonic::async_trait]
    impl pb::transport_server::Transport for ExpiryService {
        async fn unary(
            &self,
            request: tonic::Request<pb::Item>,
        ) -> Result<tonic::Response<pb::Item>, Status> {
            assert_eq!(grpc_authenticated::<Claims, _>(&request)?.id, 7);
            Ok(tonic::Response::new(request.into_inner()))
        }
        type ServerStreamStream = Output;
        async fn server_stream(
            &self,
            request: tonic::Request<pb::Item>,
        ) -> Result<tonic::Response<Output>, Status> {
            assert_eq!(grpc_authenticated::<Claims, _>(&request)?.id, 7);
            assert_eq!(
                grpc_verified_token::<Claims, _>(&request)?.claims.custom.id,
                7
            );
            let receiver = self.input.lock().await.take().unwrap();
            Ok(tonic::Response::new(Box::pin(
                streams::wrappers::ReceiverStream::new(receiver),
            )))
        }
        async fn client_stream(
            &self,
            _: tonic::Request<tonic::Streaming<pb::Item>>,
        ) -> Result<tonic::Response<pb::Summary>, Status> {
            unreachable!()
        }
        type BidiStream = Output;
        async fn bidi(
            &self,
            _: tonic::Request<tonic::Streaming<pb::Item>>,
        ) -> Result<tonic::Response<Output>, Status> {
            unreachable!()
        }
    }
    #[furnace_rs_core::cauldron(providers = [JwtService])]
    struct ExpiryApp;
    #[tokio::test]
    async fn admitted_stream_survives_token_expiry_but_next_call_fails() {
        let config = ConfigBuilder::new()
            .source(MapSource::new(
                "test",
                [("passport.secret", "sufficiently-long-test-secret-for-hs256")],
            ))
            .build()
            .unwrap();
        let jwt = JwtService::from_config(&config).unwrap();
        let token = jwt
            .sign(
                Claims { id: 7 },
                JwtSignOptions::access(Duration::from_secs(3600)),
            )
            .unwrap();
        let decoded = jwt.decode_unverified::<Claims>(&token).unwrap();
        NOW.store(decoded.registered.issued_at, Ordering::SeqCst);
        let mut builder = Furnace::builder_with_config(config);
        builder.root::<ExpiryApp>().unwrap();
        builder.provide(jwt).unwrap();
        let app = builder.build().await.unwrap();
        let guards = [crate::common::passport::__private::ScopedGuard {
            guard: &GUARD,
            occurrence: None,
            context_cauldron: Some(TypeId::of::<ExpiryApp>()),
        }];
        let preflight =
            PassportStrategyCatalog::preflight_scoped(app.cauldron_graph(), &guards).unwrap();
        let binding = preflight.bindings()[0].clone();
        let (send, receive) = tokio::sync::mpsc::channel(2);
        let server = pb::transport_server::TransportServer::new(ExpiryService {
            input: Arc::new(tokio::sync::Mutex::new(Some(receive))),
        });
        let bindings = ["ServerStream", "Unary"]
            .map(|name| {
                (
                    format!("/furnace.test.v1.Transport/{name}"),
                    Some(binding.clone()),
                )
            })
            .into_iter()
            .collect();
        let mut client = pb::transport_client::TransportClient::new(GrpcPassportService::new(
            server,
            app.context().clone(),
            bindings,
        ));
        let request = || {
            let mut req = tonic::Request::new(pb::Item { value: 0 });
            req.metadata_mut()
                .insert("authorization", format!("Bearer {token}").parse().unwrap());
            req
        };
        let mut output = client.server_stream(request()).await.unwrap().into_inner();
        NOW.store(decoded.registered.expires_at, Ordering::SeqCst);
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(
            decoded.registered.expires_at - decoded.registered.issued_at,
        ))
        .await;
        tokio::time::resume();
        send.send(Ok(pb::Item { value: 13 })).await.unwrap();
        drop(send);
        assert_eq!(output.message().await.unwrap().unwrap().value, 13);
        assert!(output.message().await.unwrap().is_none());
        assert_eq!(
            client.unary(request()).await.unwrap_err().code(),
            tonic::Code::Unauthenticated
        );
    }
    #[derive(Clone)]
    struct UnknownService(Arc<std::sync::atomic::AtomicUsize>);
    #[tonic::async_trait]
    impl pb::hidden_server::Hidden for UnknownService {
        async fn secret(
            &self,
            request: tonic::Request<pb::Item>,
        ) -> Result<tonic::Response<pb::Item>, Status> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(tonic::Response::new(request.into_inner()))
        }
    }
    #[tokio::test]
    async fn operation_missing_policy_metadata_never_dispatches() {
        let config = ConfigBuilder::new()
            .source(MapSource::new(
                "test",
                [("passport.secret", "sufficiently-long-test-secret-for-hs256")],
            ))
            .build()
            .unwrap();
        let mut builder = Furnace::builder_with_config(config);
        builder.root::<ExpiryApp>().unwrap();
        let app = builder.build().await.unwrap();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let server = pb::hidden_server::HiddenServer::new(UnknownService(calls.clone()));
        let mut client = pb::hidden_client::HiddenClient::new(GrpcPassportService::new(
            server,
            app.context().clone(),
            BTreeMap::new(),
        ));
        let error = client.secret(pb::Item { value: 1 }).await.unwrap_err();
        assert_eq!(error.code(), tonic::Code::Unimplemented);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
