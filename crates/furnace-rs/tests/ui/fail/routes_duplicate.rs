#[furnace_rs::routes(prefix = "/users")]
trait Routes {
    #[furnace_rs::get("/:id")]
    async fn first(&self);

    #[furnace_rs::get("/:id")]
    async fn second(&self);
}

fn main() {}
