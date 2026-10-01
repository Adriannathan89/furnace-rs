use furnace_rs::cauldron;

struct NotACauldron;

#[cauldron]
struct AppCauldron;

impl furnace_rs::core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.import(NotACauldron)
    }
}


fn main() {}
