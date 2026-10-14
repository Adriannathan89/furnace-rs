use furnace_rs::prelude::*;
use furnace_grpc_test_support::pb;
#[burner]
struct Credentials;
impl GrpcTokenProvider for Credentials {
    async fn token(&self) -> furnace_rs::common::GrpcClientResult<furnace_rs::core::Secret<String>> { unreachable!() }
}
#[grpc_client(name = "users", client = pb::hidden_client::HiddenClient, token_provider = Credentials)]
struct Client;
#[cauldron(root, providers = [Client])]
struct App;
fn main() {}
