use furnace_rs::common::*;

struct UserPrincipal;

#[guard(strategy = "JWT", principal = UserPrincipal)]
struct UserPolicy;

fn main() {}
