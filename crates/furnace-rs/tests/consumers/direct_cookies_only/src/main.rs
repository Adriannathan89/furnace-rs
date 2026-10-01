use ::furnace::prelude::*;
#[controller]
struct Controller;
impl Sealable for Controller { fn seals() -> SealRegistration<Self> { SealRegistration::new() } }
#[controller(route = "/user")]
impl Controller { #[get] fn index(&self) -> &'static str { "ok" } }
fn main() {}
