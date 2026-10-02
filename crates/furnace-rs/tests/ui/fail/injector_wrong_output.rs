use furnace_rs::prelude::*;
struct Implementation;
impl Injector for Implementation {
    type Dependencies = ();
    async fn inject((): ()) -> furnace_rs::core::Result<Self> {
        Ok(Self)
    }
}
#[cauldron]
struct App;
impl Cauldron for App {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide_with::<String, Implementation>()
    }
}
fn main() {}
