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

#[cauldron(root, controllers = [HelloController])]
struct AppCauldron;

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
