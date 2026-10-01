//! Conventional MADS startup with the opt-in native SeaORM module.
#![cfg(feature = "sea-orm-postgres")]

use mads::prelude::*;
use mads_persistence::sea_orm::DatabaseModule;

mod delivery {
    use mads::prelude::*;

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

    #[furnace]
    pub struct HealthModule;

    impl mads_core::Furnace for HealthModule {
        fn register(self) -> mads_core::FurnaceRegistration<Self> {
            self.controller::<HealthController>()
        }
    }
}

#[furnace]
struct AppModule;

impl mads_core::Furnace for AppModule {
    fn register(self) -> mads_core::FurnaceRegistration<Self> {
        self.import(DatabaseModule).import(delivery::HealthModule)
    }
}

#[mads::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Mads::burn::<AppModule>().await
}
