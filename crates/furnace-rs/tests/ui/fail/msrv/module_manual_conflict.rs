use furnace_rs::{Cauldron, CauldronRegistration};
#[furnace_rs::cauldron(providers = [])]
struct App;
impl Cauldron for App {
    fn register(self) -> CauldronRegistration<Self> {
        CauldronRegistration::new(self)
    }
}
fn main() {}
