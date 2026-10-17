use furnace_rs::prelude::*;
use furnace_rs::{ClaimsPrincipal, PassportPrincipal};
use furnace_grpc_test_support::{pb, FILE_DESCRIPTOR_SET};
#[derive(serde::Deserialize)]
struct Claims { id: u64 }
impl PassportPrincipal for Claims {
    fn has_role(&self, _: &str) -> bool { false }
    fn has_permission(&self, _: &str) -> bool { false }
}
#[guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>)]
struct Protection;
#[burner]
struct Service;
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET, guard = Protection)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Service {
    async fn secret(&self, request: tonic::Request<pb::Item>) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        let principal = grpc_authenticated::<ClaimsPrincipal<Claims>, _>(&request)?;
        let verified = grpc_verified_token::<Claims, _>(&request)?;
        assert_eq!(principal.id, verified.claims.custom.id);
        Ok(tonic::Response::new(pb::Item { value: principal.id }))
    }
}
#[cauldron(root, grpc_services = [Service])]
struct App;
fn main() {}
