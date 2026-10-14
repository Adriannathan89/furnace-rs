//! Health remains usable in the base transport package without Passport or TLS.
use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs_grpc::{
    core::{Furnace, burner, cauldron},
    grpc_service, tonic,
};
#[burner]
struct Service;
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Service {
    async fn secret(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
}
#[cauldron(root, grpc_services = [Service])]
struct App;
#[tokio::test]
async fn base_transport_health_check_and_watch_are_public() {
    let mut builder = Furnace::builder();
    builder.root::<App>().unwrap();
    let app = builder.build().await.unwrap();
    let scope = furnace_rs_grpc::GrpcApplicationScope::for_application(&app).unwrap();
    let routes = furnace_rs_grpc::build_grpc_routes(&app).unwrap();
    let (routes, mut health) =
        furnace_rs_grpc::install_grpc_infrastructure(&app, &scope, routes).unwrap();
    health.set_serving(false).await;
    let mut client = tonic_health::pb::health_client::HealthClient::new(routes.prepare());
    let request = || tonic_health::pb::HealthCheckRequest {
        service: "furnace.test.v1.Hidden".into(),
    };
    assert_eq!(
        client.check(request()).await.unwrap().into_inner().status,
        2
    );
    let mut output = client.watch(request()).await.unwrap().into_inner();
    assert_eq!(output.message().await.unwrap().unwrap().status, 2);
    health.set_serving(true).await;
    assert_eq!(output.message().await.unwrap().unwrap().status, 1);
}
#[cfg(not(feature = "jwt"))]
#[tokio::test]
async fn enabled_reflection_without_jwt_fails_preflight() {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [("grpc.reflection.enabled", "true")],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    let error = match builder.build().await {
        Ok(_) => panic!("reflection enabled without JWT"),
        Err(error) => error,
    };
    assert_eq!(error.code(), furnace_rs_grpc::FURNACE152);
}

#[cfg(not(feature = "jwt"))]
use furnace_rs_grpc::core::{ConfigBuilder, MapSource};
