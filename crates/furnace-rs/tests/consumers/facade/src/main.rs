//! Verifies attribute expansion through a facade-only dependency.

use furnace_rs::prelude::*;

#[cauldron(providers = [Repository], controllers = [Controller])]
struct AppCauldron;

#[storage]
struct Repository {
    #[inject]
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
