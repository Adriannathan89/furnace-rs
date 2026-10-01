use furnace_rs::common::*;

#[routes]
#[guard(skip)]
trait UserRoutes {
    #[get("/profile")]
    async fn profile(&self);
}

fn main() {}
