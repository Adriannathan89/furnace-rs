use furnace_rs_core::{cauldron, Cauldron, CauldronRegistration};
#[cauldron] struct Legacy;
impl Cauldron for Legacy { fn register(self) -> CauldronRegistration<Self> { CauldronRegistration::new(self) } }
#[cauldron(root, imports=[Legacy])] struct App;
fn main() {}
