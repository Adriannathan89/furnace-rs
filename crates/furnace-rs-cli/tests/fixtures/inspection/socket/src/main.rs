use furnace::{core::{cauldron,Injector,Furnace},runtime::FurnaceBurnExt,socket_gateway,SocketContext,SocketResult};
struct Gateway;
impl Injector for Gateway{type Dependencies=();async fn inject(_:())->furnace::core::Result<Self>{panic!("inspection must not construct a gateway")}}
#[socket_gateway(path="/chat",mode="events")]
impl Gateway {#[on_event("echo")]async fn echo(&self,c:SocketContext,data:serde_json::Value)->SocketResult<()>{c.emit("echo",data).await}}
#[cauldron(root,socket_gateways=[Gateway])]struct App;
#[tokio::main]async fn main(){Furnace::burn::<App>().await.unwrap();}
