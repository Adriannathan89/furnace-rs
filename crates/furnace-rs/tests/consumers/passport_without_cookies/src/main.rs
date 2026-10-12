#[furnace_rs::guard(strategy = "custom", principal = Principal, source = cookie("session"))]
struct Policy;
fn main() {}
