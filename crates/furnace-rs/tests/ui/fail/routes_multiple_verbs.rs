#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::get("/")]
    #[furnace_rs::post("/")]
    async fn index(&self);
}

fn main() {}
