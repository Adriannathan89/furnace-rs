use furnace_rs::{guard, PassportPrincipal};
#[derive(PassportPrincipal)]
struct Principal { subject: String }
#[guard(strategy = "custom", principal = Principal)]
struct Policy<T>(T);
fn main() {}
