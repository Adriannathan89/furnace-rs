#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::get("index")]
    async fn index(&self);
}

fn main() {}
