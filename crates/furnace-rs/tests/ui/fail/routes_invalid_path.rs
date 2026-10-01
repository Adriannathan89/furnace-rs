#[furnace_rs::routes(prefix = "users")]
trait Routes {
    #[furnace_rs::get("index")]
    async fn index(&self);
}

fn main() {}
