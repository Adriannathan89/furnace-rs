use furnace_rs::grpc_client;
#[grpc_client(name = "1users", client = MissingClient)]
struct Client;
fn main() {}
