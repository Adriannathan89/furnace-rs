use ::furnace::{cauldron, Cauldron, CauldronRegistration, Furnace};
#[cauldron]
struct AppCauldron;
impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> { CauldronRegistration::new(self) }
}
fn main() {
    let mut builder = Furnace::builder();
    builder.root::<AppCauldron>().unwrap();
    assert!(builder.analyze().is_valid());
}
