mod controller;
mod model;
mod repository;
mod service;
mod traits;

use furnace_rs::prelude::*;

#[cauldron]
pub struct AuthCauldron;

impl furnace_rs::core::Cauldron for AuthCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.provide_with::<std::sync::Arc<dyn traits::UserRepository>, repository::DemoUserRepository>()
            .provide::<service::AuthServiceImpl>()
            .provide_with::<std::sync::Arc<dyn traits::AuthService>, service::AuthServiceImpl>()
            .provide::<service::DemoJwtStrategy>()
            .controller::<controller::AuthController>()
            .controller::<controller::ProfileController>()
    }
}
