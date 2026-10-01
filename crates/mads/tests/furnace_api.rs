//! Public furnace vocabulary and standard startup contract.
#![cfg(all(feature = "http", feature = "runtime-tokio"))]
use mads::prelude::*;
#[mads::furnace]
struct Empty;
impl Furnace for Empty {
    fn register(self) -> FurnaceRegistration<Self> {
        FurnaceRegistration::new(self)
    }
}
#[tokio::test]
async fn burn_rejects_an_application_without_routes_before_binding() {
    fn assert_send<T: Send>(_: &T) {}
    let startup = Mads::burn::<Empty>();
    assert_send(&startup);
    assert!(startup.await.is_err());
}
