use furnace_rs::prelude::*;
struct Implementation;
impl Injector for Implementation {
    type Dependencies = ();
    async fn inject((): ()) -> furnace_rs::core::Result<Self> {
        Ok(Self)
    }
}
#[cauldron(providers = [String => Implementation])]
struct App;
fn main() {}
