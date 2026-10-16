use furnace_rs::prelude::*;
struct Gateway;
#[socket_gateway(path="/chat",mode="events")]
impl Gateway{}
fn main(){}
