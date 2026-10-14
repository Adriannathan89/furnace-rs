use kernel::{burner, cauldron};
use web::{grpc_service, tonic};
use furnace_grpc_test_support::{pb, FILE_DESCRIPTOR_SET};

#[burner]
struct Dependency;
#[burner]
struct Service { #[inject] dependency: Dependency }
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Service {
    #[grpc_method(name = "Secret", public)]
    async fn secret(&self, _: tonic::Request<pb::Item>) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        let _ = &self.dependency;
        Err(tonic::Status::unimplemented("test"))
    }
}
#[cauldron(providers = [Dependency], exports = [Dependency])]
struct Shared;
#[cauldron(root, imports = [Shared], grpc_services = [Service])]
struct App;
fn main() {}
