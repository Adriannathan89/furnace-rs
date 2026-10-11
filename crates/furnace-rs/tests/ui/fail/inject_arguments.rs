#[furnace_rs::burner]
struct Service { #[inject(token = "value")] value: String }
fn main() {}
