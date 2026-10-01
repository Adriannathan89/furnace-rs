mod controller;
mod model;
mod provider;
mod repository;
mod service;
mod traits;

use furnace_rs::prelude::*;

#[cauldron]
pub struct AuthCauldron;

impl furnace_rs::core::Cauldron for AuthCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.provide::<std::sync::Arc<dyn traits::UserRepository>>()
            .provide::<service::AuthServiceImpl>()
            .provide::<std::sync::Arc<dyn traits::AuthService>>()
            .provide::<service::DemoJwtStrategy>()
            .controller::<controller::AuthController>()
    }
}
