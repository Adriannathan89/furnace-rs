//! Verifies attribute expansion through a facade-only dependency.

use mads::prelude::*;

#[furnace]
struct AppModule;

impl mads::core::Furnace for AppModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
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
