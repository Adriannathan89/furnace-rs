use furnace_rs::prelude::*;
struct Gateway;
#[socket_gateway(path="/raw",mode="raw")]
impl Gateway{#[on_event("echo")]async fn event(&self,_:SocketContext,_:serde_json::Value)->SocketResult<()>{Ok(())}}
fn main(){}
