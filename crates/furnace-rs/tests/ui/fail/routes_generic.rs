#[furnace_rs::routes]
trait Routes<T> {
    #[furnace_rs::get("/")]
    async fn index(&self, value: T);
}

fn main() {}
