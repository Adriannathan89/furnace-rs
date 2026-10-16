use furnace_rs::prelude::*;
struct Gateway;
impl Injector for Gateway{type Dependencies=();async fn inject(_:())->furnace_rs::core::Result<Self>{Ok(Self)}}
#[socket_gateway(path="/chat",mode="events",processing="concurrent",max_concurrency=2)]
impl Gateway{
    #[cfg_attr(all(), inline)]
    #[on_event("echo")]async fn echo(&self,c:SocketContext,data:serde_json::Value)->SocketResult<()>{c.emit("echo",data).await}
    #[cfg(any())]#[on_event("excluded")]async fn excluded(&self,_:SocketContext,_:serde_json::Value)->SocketResult<()>{unreachable!()}
}
#[cauldron(root,socket_gateways=[Gateway])]struct App;
fn main(){}
