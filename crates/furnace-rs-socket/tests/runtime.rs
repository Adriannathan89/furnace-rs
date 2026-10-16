//! Listener selection rejects invalid applications before provider startup.
use furnace_rs_core::{Cauldron, CauldronRegistration, ConfigBuilder, MapSource};
#[furnace_rs_core::cauldron]
struct Empty;
impl Cauldron for Empty {
    fn register(self) -> CauldronRegistration<Self> {
        CauldronRegistration::new(self)
    }
}
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
