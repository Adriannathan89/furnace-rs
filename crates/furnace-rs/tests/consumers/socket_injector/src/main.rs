use furnace_rs::prelude::*;

struct Gateway;

#[cfg(not(feature = "missing-injector"))]
impl Injector for Gateway {
    type Dependencies = ();

    async fn inject(_: ()) -> furnace_rs::core::Result<Self> {
        Ok(Self)
    }
}

#[socket_gateway(path = "/chat", mode = "events")]
impl Gateway {}

fn main() {}
