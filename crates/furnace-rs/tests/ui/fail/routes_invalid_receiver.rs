#[furnace_rs::controller]
struct Routes;
impl furnace_rs::Sealable for Routes {
    fn seals() -> furnace_rs::SealRegistration<Self> { furnace_rs::SealRegistration::new() }
}

#[furnace_rs::controller]
impl Routes {
    #[furnace_rs::get("/")]
    async fn index(&mut self) {}
}

fn main() {}
