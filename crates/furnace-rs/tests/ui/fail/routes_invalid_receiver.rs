#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::get("/")]
    async fn index(&mut self);
}

fn main() {}
