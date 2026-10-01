use mads::prelude::*;

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
            "ok"
        }
    }

    #[furnace]
    pub struct HealthHttpModule;

impl mads::core::Furnace for HealthHttpModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.controller::<HealthController>()
    }
}

}

#[furnace]
struct AppModule;

impl mads::core::Furnace for AppModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.import(delivery :: HealthHttpModule)
    }
}


#[mads::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Mads::burn::<AppModule>().await
}
