#[furnace_rs::routes]
trait Route {
    #[furnace_rs::get("/")]
    async fn index(&self);
}

#[furnace_rs::controller(routes = [Route])]
struct Controller<T> {
    value: T,
}

fn main() {}
