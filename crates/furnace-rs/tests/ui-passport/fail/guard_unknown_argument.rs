use furnace_rs::common::*;

struct UserPrincipal;

#[guard(strategy = "jwt", principal = UserPrincipal, audience = "furnace-rs")]
struct UserPolicy;

fn main() {}
