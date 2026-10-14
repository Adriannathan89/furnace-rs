use brand::prelude::*;
use furnace_grpc_test_support::{pb, FILE_DESCRIPTOR_SET};
#[derive(Clone)]
struct Service(u64);
impl Injector for Service {
    type Dependencies = ();
    async fn inject(_: ()) -> brand::core::Result<Self> { panic!("native override must suppress construction") }
}
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Service {
    async fn secret(&self, _: tonic::Request<pb::Item>) -> Result<tonic::Response<pb::Item>, tonic::Status> { Ok(tonic::Response::new(pb::Item { value: self.0 })) }
}
#[cfg(feature = "http")]
#[controller]
struct Http;
#[cfg(feature = "http")]
impl brand::Sealable for Http { fn seals() -> brand::SealRegistration<Self> { brand::SealRegistration::new() } }
#[cfg(feature = "http")]
#[controller]
impl Http { #[get("/ready")] async fn ready(&self) -> &'static str { "ready" } }
#[cfg_attr(feature = "http", cauldron(root, grpc_services = [Service], controllers = [Http]))]
#[cfg_attr(not(feature = "http"), cauldron(root, grpc_services = [Service]))]
struct App;
#[allow(dead_code)]
async fn automatic() -> Result<(), ApplicationRuntimeError> { Furnace::burn::<App>().await }
#[brand::main]
async fn main() {
    let mut builder = Furnace::builder(); builder.root::<App>().unwrap(); builder.provide(Service(17)).unwrap();
    let application = builder.build().await.unwrap(); assert_eq!(application.context().resolve::<Service>().unwrap().0, 17);
    let _routes = build_grpc_routes(&application).unwrap();
}
