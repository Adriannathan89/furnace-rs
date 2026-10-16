use furnace_rs::prelude::*;
struct Gateway;
impl Injector for Gateway{type Dependencies=();async fn inject(_:())->furnace_rs::core::Result<Self>{Ok(Self)}}
#[socket_gateway(path="/chat",mode="events")]
impl Gateway{#[on_event("echo")]async fn echo(&self,_:SocketContext,_:usize)->SocketResult<()>{Ok(())}}
fn main(){}
