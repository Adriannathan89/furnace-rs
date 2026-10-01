#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::get("/health\0check")]
    async fn health(&self);
}

fn main() {}
