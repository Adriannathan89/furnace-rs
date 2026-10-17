use furnace_rs::prelude::*;
struct Gateway;
#[socket_gateway(path="/chat",mode="events")]
impl Gateway{#[on_event("furnace.join")]async fn join(&self,_:SocketContext,_:serde_json::Value)->SocketResult<()>{Ok(())}}
fn main(){}
