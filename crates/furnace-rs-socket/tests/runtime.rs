//! Listener selection rejects invalid applications before provider startup.
use furnace_rs_core::{ConfigBuilder, MapSource};
#[furnace_rs_core::cauldron]
struct Empty;

#[tokio::test]
async fn no_runnable_socket_fails_before_start() {
    let config = ConfigBuilder::new()
        .source(
            MapSource::new("test", [("socket.address", "127.0.0.1:0")])
                .with_string_array("furnace.transports", ["socket"]),
        )
        .build()
        .unwrap();
    assert!(
        furnace_rs_socket::common::runtime::prepare_application::<Empty>(config)
            .await
            .is_err()
    );
}

struct Chat;
struct Owned;
struct Unselected;
macro_rules! provider {
    ($($ty:ty),*) => {$(
        impl furnace_rs_core::Injector for $ty {
            type Dependencies = ();
            async fn inject(_: ()) -> furnace_rs_core::Result<Self> { Ok(Self) }
        }
    )*};
}
provider!(Chat, Owned, Unselected);
#[furnace_rs_socket::socket_gateway(path = "/chat", mode = "events")]
impl Chat {
    #[on_event("echo")]
    async fn echo(
        &self,
        _: furnace_rs_socket::SocketContext,
        _: serde_json::Value,
    ) -> furnace_rs_socket::SocketResult<()> {
        Ok(())
    }
}
#[furnace_rs_socket::socket_gateway(path = "/unselected", mode = "raw")]
impl Unselected {
    #[on_text]
    async fn text(
        &self,
        _: furnace_rs_socket::SocketContext,
        _: String,
    ) -> furnace_rs_socket::SocketResult<()> {
        Ok(())
    }
}
#[furnace_rs_socket::socket_endpoint(path = "/owned")]
impl Owned {
    async fn handle(
        &self,
        _: furnace_rs_socket::SocketHandshake,
        _: furnace_rs_socket::WebSocket,
    ) -> furnace_rs_socket::SocketResult<()> {
        Ok(())
    }
}
#[furnace_rs_core::cauldron(root, socket_gateways = [Chat], socket_endpoints = [Owned])]
struct Selected;
#[tokio::test]
async fn startup_summary_lists_only_selected_gateway_and_owned_paths() {
    let config = ConfigBuilder::new()
        .source(MapSource::new("test", [("socket.address", "127.0.0.1:0")]))
        .build()
        .unwrap();
    let mut builder = furnace_rs_core::Furnace::builder_with_config(config);
    builder.root::<Selected>().unwrap();
    let app = builder.build().await.unwrap();
    let adapter = furnace_rs_core::__private::inventory::iter::<
        furnace_rs_socket::common::runtime::RuntimeTransportDescriptor,
    >
        .into_iter()
        .find(|adapter| adapter.name == "socket")
        .unwrap();
    let prepared = (adapter.prepare)(&app, None, None).await.unwrap();
    assert_eq!(
        prepared.summary,
        "Endpoints  2\n\nPATH\n────\n/chat\n/owned"
    );
}
