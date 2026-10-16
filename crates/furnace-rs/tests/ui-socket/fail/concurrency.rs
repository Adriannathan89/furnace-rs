use furnace_rs::prelude::*;
struct Gateway;
#[socket_gateway(path="/chat",mode="events",processing="concurrent",max_concurrency=0)]
impl Gateway{}
fn main(){}
