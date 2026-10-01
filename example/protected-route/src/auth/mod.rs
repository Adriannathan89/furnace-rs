mod controller;
mod model;
mod provider;
mod repository;
mod service;
mod traits;

use mads::prelude::*;

#[furnace]
pub struct AuthModule;

impl mads::core::Furnace for AuthModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.provide::<std::sync::Arc<dyn traits::UserRepository>>()
            .provide::<service::AuthServiceImpl>()
            .provide::<std::sync::Arc<dyn traits::AuthService>>()
            .provide::<service::DemoJwtStrategy>()
            .controller::<controller::AuthController>()
    }
}
