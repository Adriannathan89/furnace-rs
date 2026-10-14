use furnace_rs::prelude::*;
use furnace_grpc_test_support::{pb, FILE_DESCRIPTOR_SET};
#[burner]
struct Service;
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Service {
    #[grpc_method(name = "Secret", public)]
    #[grpc_method(name = "Secret", public)]
    async fn secret(&self, _: tonic::Request<pb::Item>) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
}
fn main() {}
