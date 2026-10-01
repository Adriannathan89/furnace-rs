use mads::prelude::*;

#[mads::routes]
trait HealthRoutes {
    #[mads::get("/health")]
    async fn health(&self) -> &'static str;
}

#[mads::controller(routes = [HealthRoutes])]
struct HealthController;

impl HealthRoutes for HealthController {
    async fn health(&self) -> &'static str {
        "healthy"
    }
}

#[mads::furnace]
struct AppModule;

impl mads::core::Furnace for AppModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.controller::<HealthController>()
    }
}


#[mads::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Mads::burn::<AppModule>().await
}
