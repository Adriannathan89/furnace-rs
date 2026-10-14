use rpc::{grpc_service, guard, ClaimsPrincipal, PassportPrincipal, tonic};
use rpc::core::{burner, cauldron};
use furnace_grpc_test_support::{pb, FILE_DESCRIPTOR_SET};
#[derive(serde::Deserialize)]
struct Claims;
impl PassportPrincipal for Claims { fn has_role(&self, _: &str) -> bool { false } fn has_permission(&self, _: &str) -> bool { false } }
#[guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>)]
struct Protection;
#[burner]
struct Service;
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET, guard = Protection)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Service {
    async fn secret(&self, request: tonic::Request<pb::Item>) -> Result<tonic::Response<pb::Item>, tonic::Status> { rpc::grpc_authenticated::<ClaimsPrincipal<Claims>, _>(&request)?; Ok(tonic::Response::new(request.into_inner())) }
}
#[rpc::grpc_reflection(guard = Protection)]
#[cauldron(root, grpc_services = [Service])]
struct App;
fn main() {}

#[allow(dead_code)]
async fn automatic_runtime() -> Result<(), rpc::ApplicationRuntimeError> { use rpc::FurnaceBurnExt; rpc::core::Furnace::burn::<App>().await }
