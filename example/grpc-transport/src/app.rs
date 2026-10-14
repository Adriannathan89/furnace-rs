use crate::{
    auth::{Reader, Tokens},
    client::Remote,
    service::Transport,
};
use furnace::prelude::*;

#[grpc_reflection(guard = Reader)]
#[cfg_attr(
    feature = "http",
    cauldron(
        root,
        providers = [JwtService, Tokens, Remote],
        grpc_services = [Transport],
        controllers = [HttpHealth]
    )
)]
#[cfg_attr(
    not(feature = "http"),
    cauldron(
        root,
        providers = [JwtService, Tokens, Remote],
        grpc_services = [Transport]
    )
)]
pub struct App;

#[cauldron(root, providers = [JwtService, Tokens, Remote])]
pub struct Consumer;

#[cfg(feature = "http")]
#[controller]
pub struct HttpHealth;

#[cfg(feature = "http")]
impl furnace::Sealable for HttpHealth {
    fn seals() -> furnace::SealRegistration<Self> {
        furnace::SealRegistration::new()
    }
}

#[cfg(feature = "http")]
#[controller]
impl HttpHealth {
    #[get("/health")]
    async fn health(&self) -> &'static str {
        "ok"
    }
}
