//! Passport admission precedes generated protobuf decoding for every RPC shape.
#![cfg(feature = "jwt")]
use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs_core::{ConfigBuilder, Furnace, MapSource, burner, cauldron};
use furnace_rs_grpc::{
    JwtClaims, JwtService, JwtSignOptions, JwtTokenKind, PassportContext, PassportError,
    PassportPrincipal, PassportResult, PassportStrategy, grpc_authenticated, grpc_service,
    grpc_verified_token, guard, passport_strategy, tonic,
};
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use tonic::codegen::tokio_stream::{self as streams, Stream};
use tower::ServiceExt;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Claims {
    id: u64,
    role: bool,
    permission: bool,
    predicate: bool,
    operational_failure: bool,
}
struct Principal(Claims);
impl PassportPrincipal for Principal {
    fn has_role(&self, _: &str) -> bool {
        self.0.role
    }
    fn has_permission(&self, _: &str) -> bool {
        self.0.permission
    }
}
fn allowed(p: &Principal) -> bool {
    p.0.predicate
}
#[burner]
struct Strategy;
#[passport_strategy(name = "grpc-test")]
impl PassportStrategy for Strategy {
    type Claims = Claims;
    type Principal = Principal;
    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;
    async fn validate(
        &self,
        ctx: &PassportContext<'_>,
        claims: &JwtClaims<Claims>,
    ) -> PassportResult<Principal> {
        for name in ["authorization", "proxy-authorization", "cookie"] {
            assert!(ctx.headers().get(name).is_none());
        }
        assert_eq!(ctx.method(), http::Method::POST);
        if ctx.headers().contains_key("x-expected-peer") {
            assert!(ctx.remote_addr().unwrap().ip().is_loopback());
        }
        assert!(ctx.uri().path().starts_with("/furnace.test.v1."));
        if claims.custom.operational_failure {
            return Err(PassportError::internal(std::io::Error::other(
                "private strategy failure",
            )));
        }
        Ok(Principal(claims.custom.clone()))
    }
}
#[guard(strategy = "grpc-test", principal = Principal, roles(any = ["reader"]), permissions(all = ["read"]), predicate = allowed)]
struct Protected;
#[guard(strategy = "grpc-test", principal = Principal)]
struct IdentityOnly;
#[derive(Clone, Default)]
struct Counters {
    calls: Arc<AtomicUsize>,
    consumed: Arc<AtomicUsize>,
}
impl furnace_rs_core::Injector for Counters {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self::default())
    }
}
#[burner]
struct Service {
    #[inject]
    counters: Counters,
}
type Output = Pin<Box<dyn Stream<Item = Result<pb::Item, tonic::Status>> + Send>>;
impl Service {
    fn admitted<T>(&self, req: &tonic::Request<T>) -> Result<(), tonic::Status> {
        let principal = grpc_authenticated::<Principal, _>(req)?;
        let verified = grpc_verified_token::<Claims, _>(req)?;
        assert_eq!(principal.0.id, verified.claims.custom.id);
        assert!(std::ptr::eq(
            &*principal,
            &**req
                .extensions()
                .get::<furnace_rs_grpc::Authenticated<Principal>>()
                .unwrap()
        ));
        assert!(std::ptr::eq(
            verified.verified(),
            req.extensions()
                .get::<furnace_rs_grpc::VerifiedToken<Claims>>()
                .unwrap()
                .verified()
        ));
        self.counters.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
#[grpc_service(server = pb::transport_server::TransportServer, descriptor = FILE_DESCRIPTOR_SET, guard = Protected)]
#[tonic::async_trait]
impl pb::transport_server::Transport for Service {
    async fn unary(
        &self,
        req: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        self.admitted(&req)?;
        Ok(tonic::Response::new(req.into_inner()))
    }
    type ServerStreamStream = Output;
    async fn server_stream(
        &self,
        req: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        self.admitted(&req)?;
        Ok(tonic::Response::new(Box::pin(streams::iter([Ok(
            req.into_inner()
        )]))))
    }
    async fn client_stream(
        &self,
        req: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<pb::Summary>, tonic::Status> {
        self.admitted(&req)?;
        let mut input = req.into_inner();
        let mut count = 0;
        while input.message().await?.is_some() {
            count += 1;
            self.counters.consumed.fetch_add(1, Ordering::SeqCst);
        }
        Ok(tonic::Response::new(pb::Summary { count, total: 0 }))
    }
    type BidiStream = Output;
    async fn bidi(
        &self,
        req: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        self.admitted(&req)?;
        let counters = self.counters.clone();
        Ok(tonic::Response::new(Box::pin(streams::StreamExt::map(
            req.into_inner(),
            move |item| {
                counters.consumed.fetch_add(1, Ordering::SeqCst);
                item
            },
        ))))
    }
}
#[cauldron(providers = [Counters, Strategy], grpc_services = [Service])]
struct App;
fn good_claims() -> Claims {
    Claims {
        id: 7,
        role: true,
        permission: true,
        predicate: true,
        operational_failure: false,
    }
}
async fn app() -> Furnace {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [("passport.secret", "sufficiently-long-test-secret-for-hs256")],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    builder.build().await.unwrap()
}
fn token(app: &Furnace, claims: &Claims, kind: JwtTokenKind) -> String {
    app.context()
        .resolve::<JwtService>()
        .unwrap()
        .sign(
            claims,
            match kind {
                JwtTokenKind::Access => JwtSignOptions::access(Duration::from_secs(3600)),
                JwtTokenKind::Refresh => JwtSignOptions::refresh(Duration::from_secs(3600)),
            },
        )
        .unwrap()
}
struct UnreadBody(Arc<AtomicUsize>);
impl http_body::Body for UnreadBody {
    type Data = tonic::codegen::Bytes;
    type Error = std::convert::Infallible;
    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(None)
    }
}
#[tokio::test]
async fn grpc_guard_admission() {
    let app = app().await;
    let counters = app.context().resolve::<Counters>().unwrap();
    let valid = token(&app, &good_claims(), JwtTokenKind::Access);
    let mut invalid_signature = valid.clone().into_bytes();
    let start = valid.rfind('.').unwrap() + 1;
    invalid_signature[start] = if invalid_signature[start] == b'A' {
        b'B'
    } else {
        b'A'
    };
    let invalid_signature = String::from_utf8(invalid_signature).unwrap();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    header.typ = Some(JwtTokenKind::Access.header_type().into());
    let expired = jsonwebtoken::encode(&header, &serde_json::json!({"exp": 1, "iat": 0, "token_use": "access", "id": 7, "role": true, "permission": true, "predicate": true, "operational_failure": false}), &jsonwebtoken::EncodingKey::from_secret(b"sufficiently-long-test-secret-for-hs256")).unwrap();
    let mut variants: Vec<(Vec<String>, tonic::Code)> = vec![
        (vec![], tonic::Code::Unauthenticated),
        (
            vec![format!("Bearer {expired}")],
            tonic::Code::Unauthenticated,
        ),
        (
            vec![format!("Bearer {invalid_signature}")],
            tonic::Code::Unauthenticated,
        ),
        (vec!["Basic invalid".into()], tonic::Code::Unauthenticated),
        (vec!["Bearer invalid".into()], tonic::Code::Unauthenticated),
        (
            vec![format!("Bearer {valid}"), format!("Bearer {valid}")],
            tonic::Code::Unauthenticated,
        ),
        (
            vec![format!(
                "Bearer {}",
                token(&app, &good_claims(), JwtTokenKind::Refresh)
            )],
            tonic::Code::Unauthenticated,
        ),
    ];
    for field in 0..4 {
        let mut claims = good_claims();
        match field {
            0 => claims.role = false,
            1 => claims.permission = false,
            2 => claims.predicate = false,
            _ => claims.operational_failure = true,
        }
        variants.push((
            vec![format!(
                "Bearer {}",
                token(&app, &claims, JwtTokenKind::Access)
            )],
            if field == 3 {
                tonic::Code::Internal
            } else {
                tonic::Code::PermissionDenied
            },
        ));
    }
    for method in ["Unary", "ServerStream", "ClientStream", "Bidi"] {
        for (credentials, code) in &variants {
            let polls = Arc::new(AtomicUsize::new(0));
            let mut request = http::Request::builder()
                .method("POST")
                .uri(format!("/furnace.test.v1.Transport/{method}"))
                .header("content-type", "application/grpc")
                .body(tonic::body::Body::new(UnreadBody(polls.clone())))
                .unwrap();
            for credential in credentials {
                request
                    .headers_mut()
                    .append("authorization", credential.parse().unwrap());
            }
            request
                .headers_mut()
                .insert("cookie", "private-cookie=secret".parse().unwrap());
            request
                .headers_mut()
                .insert("proxy-authorization", "private".parse().unwrap());
            let response = furnace_rs_grpc::build_grpc_routes(&app)
                .unwrap()
                .prepare()
                .oneshot(request)
                .await
                .unwrap();
            let status = tonic::Status::from_header_map(response.headers()).expect("gRPC status");
            assert_eq!(status.code(), *code, "method {method}");
            assert_eq!(
                status.message(),
                match code {
                    tonic::Code::Unauthenticated => "unauthenticated",
                    tonic::Code::PermissionDenied => "permission denied",
                    _ => "internal server error",
                }
            );
            assert_eq!(counters.calls.load(Ordering::SeqCst), 0);
            assert_eq!(counters.consumed.load(Ordering::SeqCst), 0);
            assert_eq!(polls.load(Ordering::SeqCst), 0);
        }
    }
}

#[burner]
struct OverrideService {
    #[inject]
    counters: Counters,
}
#[grpc_service(server = pb::transport_server::TransportServer, descriptor = FILE_DESCRIPTOR_SET, guard = Protected)]
#[tonic::async_trait]
impl pb::transport_server::Transport for OverrideService {
    #[grpc_method(name = "Unary", public)]
    async fn unary(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        assert!(grpc_authenticated::<Principal, _>(&request).is_err());
        self.counters.calls.fetch_add(1, Ordering::SeqCst);
        Ok(tonic::Response::new(request.into_inner()))
    }
    type ServerStreamStream = Output;
    #[grpc_method(name = "ServerStream", guard = IdentityOnly)]
    async fn server_stream(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        let principal = grpc_authenticated::<Principal, _>(&request)?;
        let verified = grpc_verified_token::<Claims, _>(&request)?;
        assert_eq!(principal.0.id, 7);
        assert_eq!(verified.claims.custom.id, 7);
        assert!(!principal.0.role);
        assert!(!principal.0.permission);
        assert!(!principal.0.predicate);
        self.counters.calls.fetch_add(1, Ordering::SeqCst);
        Ok(tonic::Response::new(Box::pin(streams::iter([Ok(
            request.into_inner()
        )]))))
    }
    async fn client_stream(
        &self,
        _: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<pb::Summary>, tonic::Status> {
        panic!("rejected call entered handler")
    }
    type BidiStream = Output;
    async fn bidi(
        &self,
        _: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        panic!("rejected call entered handler")
    }
}
#[cauldron(providers = [Counters, Strategy], grpc_services = [OverrideService])]
struct OverrideApp;

async fn configured_app<M: furnace_rs_core::Cauldron>() -> Furnace {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [("passport.secret", "sufficiently-long-test-secret-for-hs256")],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<M>().unwrap();
    builder.build().await.unwrap()
}
async fn native_call(app: &Furnace, public: bool) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let routes = furnace_rs_grpc::build_grpc_routes(app).unwrap();
    let (stop, signal) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_routes(routes)
            .serve_with_incoming_shutdown(
                streams::wrappers::TcpListenerStream::new(listener),
                async {
                    let _ = signal.await;
                },
            )
            .await
            .unwrap();
    });
    let mut client = pb::transport_client::TransportClient::connect(format!("http://{address}"))
        .await
        .unwrap();
    if public {
        assert_eq!(
            client
                .unary(pb::Item { value: 9 })
                .await
                .unwrap()
                .into_inner()
                .value,
            9
        );
    } else {
        let mut claims = good_claims();
        claims.role = false;
        claims.permission = false;
        claims.predicate = false;
        let mut request = tonic::Request::new(pb::Item { value: 7 });
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token(app, &claims, JwtTokenKind::Access))
                .parse()
                .unwrap(),
        );
        request
            .metadata_mut()
            .insert("cookie", "private-cookie=secret".parse().unwrap());
        request
            .metadata_mut()
            .insert("proxy-authorization", "secret".parse().unwrap());
        let mut output = client.server_stream(request).await.unwrap().into_inner();
        assert_eq!(output.message().await.unwrap().unwrap().value, 7);
        assert!(output.message().await.unwrap().is_none());
    }
    drop(client);
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
}
#[tokio::test]
async fn method_policy_replaces_service_policy() {
    let app = configured_app::<OverrideApp>().await;
    native_call(&app, false).await;
}
#[tokio::test]
async fn explicit_public_method_bypasses_service_guard() {
    let app = configured_app::<OverrideApp>().await;
    native_call(&app, true).await;
}
#[tokio::test]
async fn grpc_strategy_context_is_sanitized() {
    let app = configured_app::<OverrideApp>().await;
    native_call(&app, false).await;
}

struct ConstructorSentinel;
impl furnace_rs_core::Injector for ConstructorSentinel {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        panic!("preflight failure must precede constructors")
    }
}
#[cauldron(providers = [Counters, ConstructorSentinel], grpc_services = [Service])]
struct MissingStrategyApp;
#[tokio::test]
async fn unreachable_strategy_cannot_satisfy_grpc_guard() {
    let mut builder = Furnace::builder();
    builder.root::<MissingStrategyApp>().unwrap();
    let error = match builder.build().await {
        Ok(_) => panic!("unreachable strategy accepted"),
        Err(error) => error,
    };
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.code() == furnace_rs_grpc::FURNACE130)
    );
}

#[tokio::test]
async fn admitted_calls_retain_typed_principals_and_claims_for_all_shapes() {
    let app = app().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let routes = furnace_rs_grpc::build_grpc_routes(&app).unwrap();
    let (stop, signal) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_routes(routes)
            .serve_with_incoming_shutdown(
                streams::wrappers::TcpListenerStream::new(listener),
                async {
                    let _ = signal.await;
                },
            )
            .await
            .unwrap();
    });
    let mut client = pb::transport_client::TransportClient::connect(format!("http://{address}"))
        .await
        .unwrap();
    let valid = token(&app, &good_claims(), JwtTokenKind::Access);
    let unary_request = || {
        let mut request = tonic::Request::new(pb::Item { value: 7 });
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer {valid}").parse().unwrap());
        request
    };
    let stream_request = || {
        let mut request = tonic::Request::new(streams::iter([pb::Item { value: 13 }]));
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer {valid}").parse().unwrap());
        request
    };
    assert_eq!(
        client
            .unary(unary_request())
            .await
            .unwrap()
            .into_inner()
            .value,
        7
    );
    let mut output = client
        .server_stream(unary_request())
        .await
        .unwrap()
        .into_inner();
    assert_eq!(output.message().await.unwrap().unwrap().value, 7);
    assert!(output.message().await.unwrap().is_none());
    drop(output);
    assert_eq!(
        client
            .client_stream(stream_request())
            .await
            .unwrap()
            .into_inner()
            .count,
        1
    );
    let mut output = client.bidi(stream_request()).await.unwrap().into_inner();
    assert_eq!(output.message().await.unwrap().unwrap().value, 13);
    assert!(output.message().await.unwrap().is_none());
    drop(output);
    let counters = app.context().resolve::<Counters>().unwrap();
    assert_eq!(counters.calls.load(Ordering::SeqCst), 4);
    assert_eq!(counters.consumed.load(Ordering::SeqCst), 2);
    drop(client);
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
}

#[burner]
struct StaleService {
    #[inject]
    counters: Counters,
}
#[grpc_service(server = pb::transport_server::TransportServer, descriptor = furnace_grpc_test_support::STALE_DESCRIPTOR_SET, guard = IdentityOnly)]
#[tonic::async_trait]
impl pb::transport_server::Transport for StaleService {
    async fn unary(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        self.counters.calls.fetch_add(1, Ordering::SeqCst);
        Err(tonic::Status::unimplemented("test"))
    }
    type ServerStreamStream = Output;
    async fn server_stream(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        unreachable!()
    }
    async fn client_stream(
        &self,
        _: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<pb::Summary>, tonic::Status> {
        unreachable!()
    }
    type BidiStream = Output;
    async fn bidi(
        &self,
        _: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        unreachable!()
    }
}
#[cauldron(root, providers = [Counters, Strategy, JwtService], grpc_services = [StaleService])]
struct StaleRoot;
#[test]
fn stale_descriptor_missing_callable_rpc_is_rejected_before_construction() {
    let graph = furnace_rs_core::__private::build_cauldron_graph::<StaleRoot>().unwrap();
    let error =
        furnace_rs_grpc::GrpcApplicationScope::for_cauldron_graph(Some(&graph)).unwrap_err();
    assert_eq!(error.code(), furnace_rs_grpc::FURNACE150);
    assert!(error.to_string().contains("methods"));
}
