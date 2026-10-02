use furnace_rs::prelude::*;

#[controller]
struct HelloController;

#[controller(route = "/")]
impl HelloController {
    #[get]
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
