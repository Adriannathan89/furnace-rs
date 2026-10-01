use mads::prelude::*;

#[routes]
trait HelloRoutes {
    #[get("/")]
    async fn hello(&self) -> &'static str;
}

#[controller(routes = [HelloRoutes])]
struct HelloController;

impl HelloRoutes for HelloController {
    async fn hello(&self) -> &'static str {
        "Hello, world!"
    }
}

#[furnace]
struct AppModule;

impl mads::core::Furnace for AppModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.controller::<HelloController>()
    }
}

#[mads::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Mads::burn::<AppModule>().await
}
