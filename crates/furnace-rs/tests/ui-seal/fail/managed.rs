use furnace_rs::{guard, PassportPrincipal};
#[derive(PassportPrincipal)]
struct Principal { subject: String }
#[guard(strategy = "custom", principal = Principal)]
#[furnace_rs::burner]
struct Policy;
fn main() {}
