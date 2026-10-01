#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::get("/:9id")]
    async fn get_user(&self);
}

fn main() {}
