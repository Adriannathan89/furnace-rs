use furnace::{tonic, grpc_service, grpc_client, guard, ClaimsPrincipal, PassportPrincipal, FurnaceBurnExt};
use furnace::core::{cauldron, Furnace, Injector};
mod pb { tonic::include_proto!("inspection.v1"); }
const DESCRIPTOR: &[u8] = tonic::include_file_descriptor_set!("service");
#[derive(serde::Deserialize)]
struct Claims;
impl PassportPrincipal for Claims { fn has_role(&self, _: &str) -> bool { true } fn has_permission(&self, _: &str) -> bool { true } }
fn allowed(_: &ClaimsPrincipal<Claims>) -> bool { true }
#[guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>, roles(any = ["reader", "admin"]), permissions(all = ["transport:read"]), predicate = allowed)]
struct Protection;
#[derive(Clone)]
struct Service;
impl Injector for Service {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace::core::Result<Self> { panic!("inspection constructed a provider") }
}
type Output = std::pin::Pin<Box<dyn tonic::codegen::tokio_stream::Stream<Item = Result<pb::Item, tonic::Status>> + Send>>;
#[grpc_service(server = pb::transport_server::TransportServer, descriptor = DESCRIPTOR, guard = Protection)]
#[tonic::async_trait]
impl pb::transport_server::Transport for Service {
    #[grpc_method(name = "Unary", public)]
    async fn unary(&self, _: tonic::Request<pb::Item>) -> Result<tonic::Response<pb::Item>, tonic::Status> { unreachable!() }
    type ServerStreamStream = Output;
    async fn server_stream(&self, _: tonic::Request<pb::Item>) -> Result<tonic::Response<Output>, tonic::Status> { unreachable!() }
    async fn client_stream(&self, _: tonic::Request<tonic::Streaming<pb::Item>>) -> Result<tonic::Response<pb::Item>, tonic::Status> { unreachable!() }
    type BidiStream = Output;
    async fn bidi(&self, _: tonic::Request<tonic::Streaming<pb::Item>>) -> Result<tonic::Response<Output>, tonic::Status> { unreachable!() }
}
#[grpc_client(name = "remote", client = pb::transport_client::TransportClient)]
struct Remote;
#[cauldron(root, providers = [Remote], grpc_services = [Service])]
struct App;
#[tokio::main]
async fn main() -> Result<(), furnace::ApplicationRuntimeError> { Furnace::burn::<App>().await }
