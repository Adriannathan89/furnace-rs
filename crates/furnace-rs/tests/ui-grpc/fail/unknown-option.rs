use furnace_rs::prelude::*;
struct Service;
#[grpc_service(unknown = Service)]
impl Service {}
fn main() {}
