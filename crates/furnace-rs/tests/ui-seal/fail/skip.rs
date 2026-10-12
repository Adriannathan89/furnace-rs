use furnace_rs::{guard, PassportPrincipal};
#[derive(PassportPrincipal)]
struct Principal { subject: String }
#[guard(skip)]
struct Policy;
fn main() {}
