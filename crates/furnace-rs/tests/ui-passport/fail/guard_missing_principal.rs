use furnace_rs::common::*;

#[routes]
#[guard(strategy = "jwt")]
trait UserRoutes {
    #[get("/profile")]
    async fn profile(&self);
}

fn main() {}
