use furnace_rs::{controller, Sealable, SealRegistration};
#[controller]
struct Controller;
impl Sealable for Controller { fn seals() -> SealRegistration<Self> { SealRegistration::new() } }
trait Methods { fn run(&self); }
#[controller(route = "/")]
impl Methods for Controller { #[get] fn run(&self) {} }
fn main() {}
