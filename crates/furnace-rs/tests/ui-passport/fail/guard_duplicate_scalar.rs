use furnace_rs::common::*;

struct UserPrincipal;

#[guard(
    strategy = "jwt",
    strategy = "jwt-refresh",
    principal = UserPrincipal,
)]
struct UserPolicy;

fn main() {}
