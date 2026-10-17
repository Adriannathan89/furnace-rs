use furnace_rs::prelude::*;
use furnace_grpc_test_support::{pb, FILE_DESCRIPTOR_SET};
struct Service<T>(std::marker::PhantomData<T>);
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl<T> pb::hidden_server::Hidden for Service<T> {
    #[grpc_method(name = "Secret", public)]
    async fn secret(&self, _: tonic::Request<pb::Item>) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
}
fn main() {}
