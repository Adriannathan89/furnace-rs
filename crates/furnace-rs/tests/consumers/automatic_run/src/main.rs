use furnace_rs::prelude::*;

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
            "ok"
        }
    }

    #[cauldron]
    pub struct HealthHttpCauldron;

    impl furnace_rs::core::Cauldron for HealthHttpCauldron {
        fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
            self.controller::<HealthController>()
        }
    }
}

#[cauldron]
struct AppCauldron;

impl furnace_rs::core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.import(delivery::HealthHttpCauldron)
    }
}

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
