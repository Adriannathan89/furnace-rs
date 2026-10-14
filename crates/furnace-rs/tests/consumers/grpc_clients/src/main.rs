use web::{grpc_client, GrpcClientResult, GrpcTokenProvider, tonic};
use kernel::{burner, cauldron, Secret};
use furnace_grpc_test_support::pb;
#[burner]
struct Credentials;
impl GrpcTokenProvider for Credentials {
    async fn token(&self) -> GrpcClientResult<Secret<String>> { Ok(Secret::new("test-token".into())) }
}
#[grpc_client(name = "first", client = pb::transport_client::TransportClient, token_provider = Credentials)]
struct First;
#[grpc_client(name = "second", client = pb::transport_client::TransportClient)]
struct Second;
#[burner]
struct Caller { #[inject] first: First, #[inject] second: Second }
impl Caller {
    async fn calls(&self) -> Result<(), tonic::Status> {
        self.first.client().unary(pb::Item { value: 7 }).await?;
        self.second.client().unary(pb::Item { value: 13 }).await?;
        Ok(())
    }
}
#[cauldron(root, providers = [Credentials, First, Second, Caller])]
struct App;
fn main() {}
