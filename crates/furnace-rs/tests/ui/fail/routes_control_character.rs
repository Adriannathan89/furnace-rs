#[furnace_rs::controller]
struct Routes;
impl furnace_rs::Sealable for Routes {
    fn seals() -> furnace_rs::SealRegistration<Self> { furnace_rs::SealRegistration::new() }
}

#[furnace_rs::controller]
impl Routes {
    #[furnace_rs::get("/health\0check")]
    async fn health(&self) {}
}

fn main() {}
