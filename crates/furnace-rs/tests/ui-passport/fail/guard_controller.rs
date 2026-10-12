#[furnace_rs::common::guard(strategy = "jwt", principal = UserPrincipal)]
#[furnace_rs::controller]
struct UserController;

struct UserPrincipal;

fn main() {}
