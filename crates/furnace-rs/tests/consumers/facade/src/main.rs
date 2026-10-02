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

#[controller]
struct Controller;

impl ::furnace_rs::Sealable for Controller {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller]
impl Controller {
    #[get("/")]
    async fn index(&self) {}
}

fn main() {}
