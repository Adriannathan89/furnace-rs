//! Rejects a known FURNACE/Axum body extractor before a part extractor.

#[furnace_rs::controller]
struct Routes;
impl furnace_rs::Sealable for Routes {
    fn seals() -> furnace_rs::SealRegistration<Self> { furnace_rs::SealRegistration::new() }
}

#[furnace_rs::controller]
impl Routes {
    #[furnace_rs::post("/:id")]
    async fn create(&self, payload: furnace_rs_common::Json<String>, id: furnace_rs::common::Path<u64>) {}
}

fn main() {}
