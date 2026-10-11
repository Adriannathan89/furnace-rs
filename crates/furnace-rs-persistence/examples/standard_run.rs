//! Conventional FURNACE startup with the opt-in native SeaORM module.
#![cfg(feature = "sea-orm-postgres")]

use furnace_rs::prelude::*;
use furnace_rs_persistence::sea_orm::DatabaseCauldron;

mod delivery {
    use furnace_rs::prelude::*;

    #[controller]
    pub struct HealthController;

    impl ::furnace_rs::Sealable for HealthController {
        fn seals() -> ::furnace_rs::SealRegistration<Self> {
            ::furnace_rs::SealRegistration::new()
        }
    }

    #[furnace_rs::controller]
    impl HealthController {
        #[get("/health")]
        async fn health(&self) -> &'static str {
            "healthy"
        }
    }

    #[cauldron(controllers = [HealthController])]
    pub struct HealthCauldron;
}

#[cauldron(root, imports = [DatabaseCauldron, LoggerCauldron, delivery::HealthCauldron])]
struct AppCauldron;

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
