use crate::{auth::Tokens, pb};
use furnace::prelude::*;
use tonic::codegen::tokio_stream;

#[grpc_client(
    name = "remote",
    client = pb::transport_client::TransportClient,
    token_provider = Tokens
)]
pub struct Remote;

/// Uses the generated API through the managed wrapper for all four RPC shapes.
pub async fn exercise(remote: &Remote) -> Result<(), tonic::Status> {
    let mut client = remote.client();
    let value = client
        .unary(pb::Item { value: 7 })
        .await?
        .into_inner()
        .value;
    if value != 7 {
        return Err(tonic::Status::data_loss("unexpected unary result"));
    }
    let mut output = client
        .server_stream(pb::Item { value: 3 })
        .await?
        .into_inner();
    for expected in 0..3 {
        if output.message().await?.map(|item| item.value) != Some(expected) {
            return Err(tonic::Status::data_loss("unexpected server stream"));
        }
    }
    if output.message().await?.is_some() {
        return Err(tonic::Status::data_loss("unexpected extra message"));
    }
    let summary = client
        .client_stream(tokio_stream::iter([
            pb::Item { value: 2 },
            pb::Item { value: 5 },
        ]))
        .await?
        .into_inner();
    if (summary.count, summary.total) != (2, 7) {
        return Err(tonic::Status::data_loss("unexpected client stream result"));
    }
    let mut output = client
        .bidi(tokio_stream::iter([
            pb::Item { value: 11 },
            pb::Item { value: 13 },
        ]))
        .await?
        .into_inner();
    for expected in [11, 13] {
        if output.message().await?.map(|item| item.value) != Some(expected) {
            return Err(tonic::Status::data_loss("unexpected bidirectional stream"));
        }
    }
    if output.message().await?.is_some() {
        return Err(tonic::Status::data_loss("unexpected extra message"));
    }
    Ok(())
}
