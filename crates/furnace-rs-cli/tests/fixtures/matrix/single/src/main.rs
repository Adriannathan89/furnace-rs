use furnace::prelude::*;

#[furnace::controller]
struct HealthController;

impl ::furnace::Sealable for HealthController {
    fn seals() -> ::furnace::SealRegistration<Self> {
        ::furnace::SealRegistration::new()
    }
}

#[furnace::controller]
impl HealthController {
    #[furnace::get("/health")]
    async fn health(&self) -> &'static str {
        "healthy"
    }
}

#[furnace::cauldron(controllers = [HealthController])]
struct AppCauldron;

#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
