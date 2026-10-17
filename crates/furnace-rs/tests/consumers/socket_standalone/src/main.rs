use socket::{socket_gateway,SocketContext,SocketResult,core::{cauldron,Injector}};
struct Gateway;
impl Injector for Gateway{type Dependencies=();async fn inject(_:())->socket::core::Result<Self>{Ok(Self)}}
#[socket_gateway(path="/chat",mode="events",guard=Protected)]
impl Gateway{#[on_event("echo")]async fn echo(&self,c:SocketContext,data:serde_json::Value)->SocketResult<()>{c.emit("echo",data).await}}
#[cauldron(root,socket_gateways=[Gateway])]struct App;
fn main(){}

#[derive(serde::Deserialize)]struct Claims{id:u64}
impl socket::common::PassportPrincipal for Claims{fn has_role(&self,_:&str)->bool{false}fn has_permission(&self,_:&str)->bool{false}}
#[socket::common::guard(strategy="jwt",principal=socket::common::ClaimsPrincipal<Claims>)]struct Protected;
