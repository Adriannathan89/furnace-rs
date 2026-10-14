use furnace_rs::grpc_client;
#[grpc_client(name = "users", client = MissingClient)]
struct Client { value: u32 }
fn main() {}
