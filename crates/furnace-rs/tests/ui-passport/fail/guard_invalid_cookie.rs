use furnace_rs::common::*;

struct UserPrincipal;

#[guard(
    strategy = "jwt",
    principal = UserPrincipal,
    source = cookie("bad;name"),
)]
struct UserPolicy;

fn main() {}
