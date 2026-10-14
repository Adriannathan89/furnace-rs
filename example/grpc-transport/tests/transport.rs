//! Exercises the documented managed clients against the supervised example server.
use furnace::grpc::common::runtime::{RuntimeTransportDescriptor, serve_prepared_transports_with};
use furnace::prelude::*;
use furnace_rs_example_grpc_transport::{
    app::{App, Consumer},
    auth::Tokens,
    client::{Remote, exercise},
    pb,
};
use std::time::Duration;
use tonic::codegen::tokio_stream;
use tonic_reflection::pb::v1::{
    ServerReflectionRequest, server_reflection_client::ServerReflectionClient,
    server_reflection_request::MessageRequest,
};

#[tokio::test]
async fn example_serves_consumes_authenticates_and_exposes_infrastructure() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let endpoint = format!("http://{address}");
    let config = ConfigBuilder::new()
        .source(furnace::core::MapSource::new(
            "synthetic example test",
            [
                (
                    "passport.secret",
                    "synthetic-local-example-test-key-0123456789".into(),
                ),
                ("grpc.clients.remote.endpoint", endpoint.clone()),
                ("grpc.reflection.enabled", "true".into()),
            ],
        ))
        .build()
        .unwrap();

    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    let app = builder.build().await.unwrap();
    let mut routes = RuntimeRoutes::new().with_grpc(build_grpc_routes(&app).unwrap());
    let mut bindings = RuntimeBindings::new().with_grpc(
        GrpcServerConfig::parse(app.context().config())
            .unwrap()
            .with_address(address),
    );

    let descriptor = furnace::core::__private::inventory::iter::<RuntimeTransportDescriptor>
        .into_iter()
        .find(|d| d.name == "grpc")
        .unwrap();
    let transport = (descriptor.prepare)(&app, Some(&mut routes), Some(&mut bindings))
        .await
        .unwrap();
    let jwt = app.context().resolve::<JwtService>().unwrap();
    let (stop, signal) = tokio::sync::oneshot::channel();
    let mut listener = Some(listener);
    let task = tokio::spawn(serve_prepared_transports_with(
        app,
        vec![transport],
        async {
            let _ = signal.await;
        },
        move |_| {
            let listener = listener.take().unwrap();
            Box::pin(async { Ok(listener) })
        },
    ));

    let consumer_config = ConfigBuilder::new()
        .source(furnace::core::MapSource::new(
            "synthetic consumer test",
            [
                (
                    "passport.secret",
                    "synthetic-local-example-test-key-0123456789".into(),
                ),
                ("grpc.clients.remote.endpoint", endpoint.clone()),
            ],
        ))
        .build()
        .unwrap();

    let mut builder = Furnace::builder_with_config(consumer_config);
    builder.root::<Consumer>().unwrap();
    let mut consumer = builder.build().await.unwrap();
    consumer.start().await.unwrap();
    let remote = consumer.context().resolve::<Remote>().unwrap();
    let tokens = consumer.context().resolve::<Tokens>().unwrap();

    exercise(&remote).await.unwrap();
    assert_eq!(tokens.calls(), 4);
    let channel = tonic::transport::Endpoint::from_shared(endpoint)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut native = pb::transport_client::TransportClient::new(channel.clone());
    assert!(native.unary(pb::Item { value: 9 }).await.is_ok());

    assert_eq!(
        native
            .server_stream(pb::Item { value: 2 })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );

    let credential = jwt
        .sign(
            furnace_rs_example_grpc_transport::auth::DemoClaims::reader(),
            JwtSignOptions::access(Duration::from_secs(60)),
        )
        .unwrap();
    let mut request = tonic::Request::new(pb::Item { value: 2 });
    set_grpc_bearer_token(&mut request, &credential).unwrap();
    let mut response = remote
        .client()
        .server_stream(request)
        .await
        .unwrap()
        .into_inner();
    while response.message().await.unwrap().is_some() {}
    assert_eq!(tokens.calls(), 4, "explicit Bearer suppresses provider");

    let mut health = tonic_health::pb::health_client::HealthClient::new(channel.clone());
    assert_eq!(
        health
            .check(tonic_health::pb::HealthCheckRequest {
                service: String::new()
            })
            .await
            .unwrap()
            .into_inner()
            .status,
        1
    );

    let mut reflection = ServerReflectionClient::new(channel);
    let query = ServerReflectionRequest {
        host: String::new(),
        message_request: Some(MessageRequest::ListServices(String::new())),
    };
    assert_eq!(
        reflection
            .server_reflection_info(tokio_stream::iter([query.clone()]))
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );

    let mut request = tonic::Request::new(tokio_stream::iter([query]));
    set_grpc_bearer_token(&mut request, &credential).unwrap();
    let mut stream = reflection
        .server_reflection_info(request)
        .await
        .unwrap()
        .into_inner();
    assert!(stream.message().await.unwrap().is_some());
    assert!(stream.message().await.unwrap().is_none());

    consumer.shutdown().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
}
