use furnace_rs::common::*;

struct UserPrincipal;

#[routes]
#[guard(strategy = "jwt", principal = UserPrincipal, audience = "furnace-rs")]
trait UserRoutes {
    #[get("/profile")]
    async fn profile(&self);
}

fn main() {}
