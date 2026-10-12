use furnace_rs::{guard, PassportPrincipal};
#[derive(PassportPrincipal)]
struct Principal { subject: String }
#[guard(strategy = "custom", principal = u32)]
struct Policy;
fn main() {}
