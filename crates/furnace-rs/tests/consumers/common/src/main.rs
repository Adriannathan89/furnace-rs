//! Verifies controller expansion through a direct common dependency.

#[furnace_rs_common::routes]
trait Routes {
    #[furnace_rs_common::get("/")]
    async fn index(&self);
}

#[furnace_rs_common::controller(routes = [Routes])]
struct Controller;

impl Routes for Controller {
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
