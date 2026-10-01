use furnace::prelude::*;

#[furnace::routes]
trait HealthRoutes {
    #[furnace::get("/health")]
    async fn health(&self) -> &'static str;
}

#[furnace::controller(routes = [HealthRoutes])]
struct HealthController;

impl HealthRoutes for HealthController {
    async fn health(&self) -> &'static str {
        "healthy"
    }
}

#[furnace::cauldron]
struct AppCauldron;

impl furnace::core::Cauldron for AppCauldron {
    fn register(self) -> furnace::core::CauldronRegistration<Self> {
        self.controller::<HealthController>()
    }
}


#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
