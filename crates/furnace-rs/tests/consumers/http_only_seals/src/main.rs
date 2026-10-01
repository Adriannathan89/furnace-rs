use furnace::{SealRegistration, Sealable};
struct Controller;
impl Sealable for Controller {
    fn seals() -> SealRegistration<Self> { SealRegistration::new() }
}
fn main() { let _ = Controller::seals().into_definition(); }
