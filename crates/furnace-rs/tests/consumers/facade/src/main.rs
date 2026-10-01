//! Verifies attribute expansion through a facade-only dependency.

use furnace_rs::prelude::*;

#[cauldron]
struct AppCauldron;

impl furnace_rs::core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.provide::<Repository>().controller::<Controller>()
    }
}


#[storage]
struct Repository {
    value: u32,
}

#[routes]
trait Routes {
    #[get("/")]
    async fn index(&self);
}

#[controller(routes = [Routes])]
struct Controller;

impl Routes for Controller {
    async fn index(&self) {}
}

fn main() {}
