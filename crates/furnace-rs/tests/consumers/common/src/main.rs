//! Verifies controller expansion through a direct common dependency.

#[furnace_rs_common::controller]
struct Controller;

impl ::furnace_rs_common::Sealable for Controller {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        ::furnace_rs_common::SealRegistration::new()
    }
}

#[furnace_rs_common::controller]
impl Controller {
    #[furnace_rs_common::get("/")]
    async fn index(&self) {}
}

async fn build_application() -> furnace_rs_common::core::Result<()> {
    let application = furnace_rs_common::core::Furnace::builder().build().await?;
    let _router = furnace_rs_common::build_router(&application)?;
    Ok(())
}

fn main() {
    let _ = build_application;
}
