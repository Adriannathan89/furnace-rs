use brand::{socket_gateway,SocketContext,SocketResult,core::{cauldron,Injector}};
struct Gateway;
impl Injector for Gateway{type Dependencies=();async fn inject(_:())->brand::core::Result<Self>{Ok(Self)}}
#[socket_gateway(path="/chat",mode="events")]
impl Gateway{#[on_event("echo")]async fn echo(&self,c:SocketContext,data:serde_json::Value)->SocketResult<()>{c.emit("echo",data).await}}
#[cauldron(root,socket_gateways=[Gateway])]struct App;
fn main(){}
