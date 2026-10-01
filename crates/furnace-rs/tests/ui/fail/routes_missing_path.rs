#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::get]
    async fn index(&self);
}

fn main() {}
