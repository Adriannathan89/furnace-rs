#[furnace_rs::controller]
struct Routes;
impl furnace_rs::Sealable for Routes {
    fn seals() -> furnace_rs::SealRegistration<Self> { furnace_rs::SealRegistration::new() }
}

#[furnace_rs::controller(route = "users")]
impl Routes {
    #[furnace_rs::get("index")]
    async fn index(&self) {}
}

fn main() {}
