//! Public health and root-scoped, authenticated reflection over native protobuf calls.
#![cfg(feature = "jwt")]
use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs_core::{ConfigBuilder, Furnace, MapSource, burner, cauldron};
use furnace_rs_grpc::{
    ClaimsPrincipal, GrpcHealth, JwtService, JwtSignOptions, PassportPrincipal, grpc_reflection,
    grpc_service, guard, install_grpc_infrastructure, tonic,
};
use prost::Message;
use std::{pin::Pin, time::Duration};
use tonic::codegen::tokio_stream::{self as streams, Stream};
#[derive(serde::Serialize, serde::Deserialize)]
struct Claims {
    reader: bool,
}
impl PassportPrincipal for Claims {
    fn has_role(&self, _: &str) -> bool {
        self.reader
    }
    fn has_permission(&self, _: &str) -> bool {
        false
    }
}
#[guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>, roles(any = ["reader"]))]
struct ReflectionGuard;
#[burner]
struct Service;
type Output = Pin<Box<dyn Stream<Item = Result<pb::Item, tonic::Status>> + Send>>;
#[grpc_service(server = pb::transport_server::TransportServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::transport_server::Transport for Service {
    async fn unary(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
    type ServerStreamStream = Output;
    async fn server_stream(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
    async fn client_stream(
        &self,
        _: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<pb::Summary>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
    type BidiStream = Output;
    async fn bidi(
        &self,
        _: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
}
#[grpc_reflection(guard = ReflectionGuard)]
#[cauldron(root, grpc_services = [Service])]
struct App;
#[cauldron(root, grpc_services = [Service])]
struct NoPolicyApp;
async fn app(enabled: bool) -> Furnace {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "reflection",
            [
                ("grpc.reflection.enabled", enabled.to_string()),
                (
                    "passport.secret",
                    "sufficiently-long-test-secret-for-hs256".into(),
                ),
            ],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    builder.build().await.unwrap()
}
struct Running {
    app: Furnace,
    channel: tonic::transport::Channel,
    health: GrpcHealth,
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}
impl Running {
    async fn start(enabled: bool) -> Self {
        let app = app(enabled).await;
        let scope = furnace_rs_grpc::GrpcApplicationScope::for_application(&app).unwrap();
        let routes = furnace_rs_grpc::build_grpc_routes(&app).unwrap();
        let (routes, mut health) = install_grpc_infrastructure(&app, &scope, routes).unwrap();
        health.set_serving(false).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
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
        let channel = tonic::transport::Endpoint::from_shared(endpoint)
            .unwrap()
            .connect()
            .await
            .unwrap();
        Self {
            app,
            channel,
            health,
            stop,
            task,
        }
    }
    fn token(&self, reader: bool) -> String {
        self.app
            .context()
            .resolve::<JwtService>()
            .unwrap()
            .sign(
                Claims { reader },
                JwtSignOptions::access(Duration::from_secs(3600)),
            )
            .unwrap()
    }
    async fn stop(self) {
        self.stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap();
    }
}
#[tokio::test]
async fn public_health_check_and_watch_follow_serving_state() {
    let mut server = Running::start(false).await;
    let mut client = tonic_health::pb::health_client::HealthClient::new(server.channel.clone());
    let request = || tonic_health::pb::HealthCheckRequest {
        service: "furnace.test.v1.Transport".into(),
    };
    assert_eq!(
        client.check(request()).await.unwrap().into_inner().status,
        2
    );
    let mut watch = client.watch(request()).await.unwrap().into_inner();
    assert_eq!(watch.message().await.unwrap().unwrap().status, 2);
    server.health.set_serving(true).await;
    assert_eq!(watch.message().await.unwrap().unwrap().status, 1);
    assert_eq!(
        client
            .check(tonic_health::pb::HealthCheckRequest {
                service: String::new()
            })
            .await
            .unwrap()
            .into_inner()
            .status,
        1
    );
    server.health.set_serving(false).await;
    assert_eq!(watch.message().await.unwrap().unwrap().status, 2);
    drop(watch);
    drop(client);
    server.stop().await;
}
async fn reflect(
    server: &Running,
    token: Option<String>,
    query: tonic_reflection::pb::v1::server_reflection_request::MessageRequest,
) -> Result<tonic_reflection::pb::v1::ServerReflectionResponse, tonic::Status> {
    let mut client =
        tonic_reflection::pb::v1::server_reflection_client::ServerReflectionClient::new(
            server.channel.clone(),
        );
    let input = tonic_reflection::pb::v1::ServerReflectionRequest {
        host: String::new(),
        message_request: Some(query),
    };
    let mut request = tonic::Request::new(streams::iter([input]));
    if let Some(token) = token {
        furnace_rs_grpc::set_grpc_bearer_token(&mut request, &token).unwrap();
    }
    client
        .server_reflection_info(request)
        .await?
        .into_inner()
        .message()
        .await?
        .ok_or_else(|| tonic::Status::internal("missing response"))
}
#[tokio::test]
async fn disabled_reflection_is_unimplemented() {
    let server = Running::start(false).await;
    let error = reflect(
        &server,
        None,
        tonic_reflection::pb::v1::server_reflection_request::MessageRequest::ListServices(
            String::new(),
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), tonic::Code::Unimplemented);
    server.stop().await;
}
#[tokio::test]
async fn reflection_guard_controls_admission_and_lists_only_selected_services() {
    use tonic_reflection::pb::v1::{
        server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
    };
    let server = Running::start(true).await;
    assert_eq!(
        reflect(&server, None, MessageRequest::ListServices(String::new()))
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(
        reflect(
            &server,
            Some(server.token(false)),
            MessageRequest::ListServices(String::new())
        )
        .await
        .unwrap_err()
        .code(),
        tonic::Code::PermissionDenied
    );
    let response = reflect(
        &server,
        Some(server.token(true)),
        MessageRequest::ListServices(String::new()),
    )
    .await
    .unwrap();
    let Some(MessageResponse::ListServicesResponse(list)) = response.message_response else {
        panic!("wrong reflection response")
    };
    let mut names: Vec<_> = list
        .service
        .into_iter()
        .map(|service| service.name)
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "furnace.test.v1.Transport",
            "grpc.health.v1.Health",
            "grpc.reflection.v1.ServerReflection",
            "grpc.reflection.v1alpha.ServerReflection"
        ]
    );
    server.stop().await;
}
#[tokio::test]
async fn reflection_file_by_name_and_symbol_exclude_hidden_service() {
    use tonic_reflection::pb::v1::{
        server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
    };
    let server = Running::start(true).await;
    for query in [
        MessageRequest::FileByFilename("transport.proto".into()),
        MessageRequest::FileContainingSymbol("furnace.test.v1.Transport".into()),
    ] {
        let response = reflect(&server, Some(server.token(true)), query)
            .await
            .unwrap();
        let Some(MessageResponse::FileDescriptorResponse(files)) = response.message_response else {
            panic!("wrong descriptor response")
        };
        let descriptors: Vec<_> = files
            .file_descriptor_proto
            .iter()
            .map(|bytes| prost_types::FileDescriptorProto::decode(bytes.as_slice()).unwrap())
            .collect();
        assert!(
            descriptors
                .iter()
                .all(|file| file.source_code_info.is_none())
        );
        assert!(
            !descriptors
                .iter()
                .flat_map(|file| &file.service)
                .any(|service| service.name.as_deref() == Some("Hidden"))
        );
        assert!(
            descriptors
                .iter()
                .flat_map(|file| &file.message_type)
                .any(|message| message.name.as_deref() == Some("Item"))
        );
    }
    let response = reflect(
        &server,
        Some(server.token(true)),
        MessageRequest::FileByFilename("shared.proto".into()),
    )
    .await
    .unwrap();
    let Some(MessageResponse::FileDescriptorResponse(files)) = response.message_response else {
        panic!("missing imported descriptor")
    };
    let imported =
        prost_types::FileDescriptorProto::decode(files.file_descriptor_proto[0].as_slice())
            .unwrap();
    assert!(
        imported
            .message_type
            .iter()
            .any(|message| message.name.as_deref() == Some("Summary"))
    );
    let error = reflect(
        &server,
        Some(server.token(true)),
        MessageRequest::FileContainingSymbol("furnace.test.v1.Hidden".into()),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), tonic::Code::NotFound);
    server.stop().await;
}
#[tokio::test]
async fn enabled_reflection_without_root_policy_fails_preconstruction() {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "reflection",
            [("grpc.reflection.enabled", "true")],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<NoPolicyApp>().unwrap();
    let error = match builder.build().await {
        Ok(_) => panic!("reflection exposed without policy"),
        Err(error) => error,
    };
    assert_eq!(error.code(), furnace_rs_grpc::FURNACE152);
}

#[tokio::test]
async fn v1alpha_reflection_uses_the_same_root_policy() {
    use tonic_reflection::pb::v1alpha::{
        ServerReflectionRequest, server_reflection_request::MessageRequest,
    };
    let server = Running::start(true).await;
    let mut client =
        tonic_reflection::pb::v1alpha::server_reflection_client::ServerReflectionClient::new(
            server.channel.clone(),
        );
    let input = || {
        streams::iter([ServerReflectionRequest {
            host: String::new(),
            message_request: Some(MessageRequest::ListServices(String::new())),
        }])
    };
    assert_eq!(
        client
            .server_reflection_info(input())
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    let mut request = tonic::Request::new(input());
    furnace_rs_grpc::set_grpc_bearer_token(&mut request, &server.token(true)).unwrap();
    let mut output = client
        .server_reflection_info(request)
        .await
        .unwrap()
        .into_inner();
    assert!(output.message().await.unwrap().is_some());
    assert!(output.message().await.unwrap().is_none());
    drop(output);
    drop(client);
    server.stop().await;
}
#[guard(strategy = "invisible-reflection-strategy", principal = ClaimsPrincipal<Claims>)]
struct InvisibleGuard;
#[grpc_reflection(guard = InvisibleGuard)]
#[cauldron(root, grpc_services = [Service])]
struct InvisibleApp;
#[tokio::test]
async fn enabled_reflection_requires_a_visible_strategy() {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "reflection",
            [
                ("grpc.reflection.enabled", "true"),
                ("passport.secret", "sufficiently-long-test-secret-for-hs256"),
            ],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<InvisibleApp>().unwrap();
    let error = match builder.build().await {
        Ok(_) => panic!("reflection accepted invisible strategy"),
        Err(error) => error,
    };
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.code() == furnace_rs_grpc::FURNACE130)
    );
}
