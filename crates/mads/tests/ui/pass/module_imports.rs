use mads::prelude::*;

#[furnace]
struct FeatureModule;

impl mads::core::Furnace for FeatureModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        mads::core::FurnaceRegistration::new(self)
    }
}


#[furnace]
struct AppModule;

impl mads::core::Furnace for AppModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.import(FeatureModule)
    }
}


fn assert_module<T: Furnace>() {}

fn main() {
    assert_module::<FeatureModule>();
    assert_module::<AppModule>();
}
