use furnace_rs::prelude::*;

#[cauldron]
struct FeatureCauldron;

#[cauldron(imports = [FeatureCauldron])]
struct AppCauldron;

fn assert_module<T: furnace_rs::core::__private::ModuleContract>() {}

fn main() {
    assert_module::<FeatureCauldron>();
    assert_module::<AppCauldron>();
}
