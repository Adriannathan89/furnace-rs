//! Qualified provider types may share their final identifier.
use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs_grpc::{GrpcService, grpc_service, tonic};

mod first {
    #[furnace_rs_core::burner]
    pub struct Implementation;
}
mod second {
    #[furnace_rs_core::burner]
    pub struct Implementation;
}

#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for first::Implementation {
    async fn secret(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Ok(tonic::Response::new(request.into_inner()))
    }
}

#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for second::Implementation {
    async fn secret(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Ok(tonic::Response::new(request.into_inner()))
    }
}

#[test]
fn qualified_services_keep_distinct_metadata() {
    assert_ne!(
        first::Implementation::grpc_descriptor().provider_type_id(),
        second::Implementation::grpc_descriptor().provider_type_id(),
    );
}
