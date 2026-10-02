use furnace_rs::{controller, Sealable, SealRegistration};
#[controller]
struct Controller;
impl Sealable for Controller { fn seals() -> SealRegistration<Self> { SealRegistration::new() } }
#[controller(route = "/")]
impl Controller { #[post] fn run(&self, body: furnace_rs_common::Json<String>, headers: furnace_rs_common::Header<()>) {} }
fn main() {}
