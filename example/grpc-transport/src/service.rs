use crate::{
    FILE_DESCRIPTOR_SET,
    auth::{DemoClaims, Reader},
    pb,
};
use furnace::prelude::*;
use std::pin::Pin;
use tonic::codegen::tokio_stream::{self as streams, Stream};

#[burner]
pub struct Transport;

type Output = Pin<Box<dyn Stream<Item = Result<pb::Item, tonic::Status>> + Send>>;

#[grpc_service(
    server = pb::transport_server::TransportServer,
    descriptor = FILE_DESCRIPTOR_SET,
    guard = Reader
)]
#[tonic::async_trait]
impl pb::transport_server::Transport for Transport {
    #[grpc_method(name = "Unary", public)]
    async fn unary(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Ok(tonic::Response::new(request.into_inner()))
    }

    type ServerStreamStream = Output;

    async fn server_stream(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        let _principal = grpc_authenticated::<ClaimsPrincipal<DemoClaims>, _>(&request)?;
        Ok(tonic::Response::new(Box::pin(streams::iter(
            (0..request.into_inner().value).map(|value| Ok(pb::Item { value })),
        ))))
    }

    async fn client_stream(
        &self,
        request: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<pb::Summary>, tonic::Status> {
        let _principal = grpc_authenticated::<ClaimsPrincipal<DemoClaims>, _>(&request)?;
        let mut input = request.into_inner();
        let (mut count, mut total) = (0_u64, 0_u64);
        while let Some(item) = input.message().await? {
            count += 1;
            total = total
                .checked_add(item.value)
                .ok_or_else(|| tonic::Status::out_of_range("sum overflow"))?;
        }
        Ok(tonic::Response::new(pb::Summary { count, total }))
    }

    type BidiStream = Output;

    async fn bidi(
        &self,
        request: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        let _principal = grpc_authenticated::<ClaimsPrincipal<DemoClaims>, _>(&request)?;
        Ok(tonic::Response::new(Box::pin(request.into_inner())))
    }
}
