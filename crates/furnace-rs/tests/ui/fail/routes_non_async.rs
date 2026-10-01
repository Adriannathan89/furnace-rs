#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::get("/")]
    fn index(&self);
}

fn main() {}
