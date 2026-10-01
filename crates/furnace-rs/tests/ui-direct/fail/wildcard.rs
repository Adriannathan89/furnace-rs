use furnace_rs::{controller, Sealable, SealRegistration};
#[controller]
struct Controller;
impl Sealable for Controller { fn seals() -> SealRegistration<Self> { SealRegistration::new() } }
#[controller(route = "/")]
impl Controller { #[get("/{*path}/end")] fn run(&self) {} }
fn main() {}
