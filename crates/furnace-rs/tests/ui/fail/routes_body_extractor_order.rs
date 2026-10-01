//! Rejects a known FURNACE/Axum body extractor before a part extractor.

#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::post("/:id")]
    async fn create(&self, payload: furnace_rs_common::Json<String>, id: furnace_rs::common::Path<u64>);
}

fn main() {}
