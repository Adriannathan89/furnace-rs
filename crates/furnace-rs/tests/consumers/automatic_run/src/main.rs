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

    #[cauldron(controllers = [HealthController])]
    pub struct HealthHttpCauldron;

}

#[cauldron(imports = [delivery::HealthHttpCauldron])]
struct AppCauldron;

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
