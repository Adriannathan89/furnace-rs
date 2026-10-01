#[furnace_rs::routes(prefix = "/users/")]
trait Routes {
    #[furnace_rs::get("//:id")]
    async fn get_user(&self);
}

fn main() {}
