use furnace_rs::prelude::*;

#[cauldron]
struct FeatureCauldron;

impl furnace_rs::core::Cauldron for FeatureCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        furnace_rs::core::CauldronRegistration::new(self)
    }
}


#[cauldron]
struct AppCauldron;

impl furnace_rs::core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.import(FeatureCauldron)
    }
}


fn assert_module<T: Cauldron>() {}

fn main() {
    assert_module::<FeatureCauldron>();
    assert_module::<AppCauldron>();
}
