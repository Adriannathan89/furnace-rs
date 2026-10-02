use furnace_rs::{guard, PassportPrincipal};
#[derive(PassportPrincipal)]
struct Principal { subject: String }
#[guard(strategy = "custom")]
struct Policy;
fn main() {}
