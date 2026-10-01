//! Confirms provider attributes require a concrete return type.

#[furnace_rs::element]
fn value() -> _ {
    String::new()
}

fn main() {}
