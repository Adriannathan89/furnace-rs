//! Rejects route verbs nested inside conditional attribute expansion.

#[furnace_rs::controller]
struct Routes;
impl furnace_rs::Sealable for Routes {
    fn seals() -> furnace_rs::SealRegistration<Self> { furnace_rs::SealRegistration::new() }
}

#[furnace_rs::controller]
impl Routes {
    #[cfg_attr(feature = "conditional-route", furnace_rs::get("/conditional"))]
    async fn conditional(&self) -> &'static str {}
}

fn main() {}
