use furnace_rs::prelude::*;
#[burner]
struct Ordinary;
#[cauldron(grpc_services = [Ordinary])]
struct App;
fn main() {}
