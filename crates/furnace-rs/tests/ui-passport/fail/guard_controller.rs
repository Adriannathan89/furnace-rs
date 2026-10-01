#[furnace_rs::common::guard(strategy = "jwt", principal = UserPrincipal)]
#[furnace_rs::controller(routes = [MissingRoutes])]
struct UserController;

struct UserPrincipal;

fn main() {}
