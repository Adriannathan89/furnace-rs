//! Conventional FURNACE startup with the opt-in native SeaORM module.
#![cfg(feature = "sea-orm-postgres")]

use furnace_rs::prelude::*;
use furnace_rs_persistence::sea_orm::DatabaseCauldron;

mod delivery {
    use furnace_rs::prelude::*;

    #[routes]
    pub trait HealthRoutes {
        #[get("/health")]
        async fn health(&self) -> &'static str;
    }

    #[controller(routes = [HealthRoutes])]
    pub struct HealthController;

    impl HealthRoutes for HealthController {
        async fn health(&self) -> &'static str {
            "healthy"
        }
    }

    #[cauldron]
    pub struct HealthCauldron;

    impl furnace_rs_core::Cauldron for HealthCauldron {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.controller::<HealthController>()
        }
    }
}

#[cauldron]
struct AppCauldron;

impl furnace_rs_core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        self.import(DatabaseCauldron)
            .import(delivery::HealthCauldron)
    }
}

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
