use furnace_rs::prelude::*;

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

#[cauldron]
struct AppCauldron;

impl furnace_rs::core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.controller::<HelloController>()
    }
}

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
