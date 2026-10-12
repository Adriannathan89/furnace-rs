mod controller;
mod model;
mod repository;
mod service;
mod traits;

use furnace_rs::prelude::*;

#[cauldron(
    providers = [
        std::sync::Arc<dyn traits::UserRepository> => repository::DemoUserRepository,
        JwtService,
        service::AuthServiceImpl,
        std::sync::Arc<dyn traits::AuthService> => service::AuthServiceImpl,
        service::DemoJwtStrategy,
    ],
    controllers = [controller::AuthController, controller::ProfileController],
)]
pub struct AuthCauldron;
