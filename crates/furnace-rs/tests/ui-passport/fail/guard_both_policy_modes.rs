use furnace_rs::common::*;

struct UserPrincipal;

#[guard(
    strategy = "jwt",
    principal = UserPrincipal,
    roles(any = ["user"], all = ["admin"]),
)]
struct UserPolicy;

fn main() {}
