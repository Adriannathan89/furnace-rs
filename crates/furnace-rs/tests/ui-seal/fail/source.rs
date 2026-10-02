use furnace_rs::{guard, PassportPrincipal};
#[derive(PassportPrincipal)]
struct Principal { subject: String }
#[guard(strategy = "custom", principal = Principal, source = header("secret"))]
struct Policy;
fn main() {}
