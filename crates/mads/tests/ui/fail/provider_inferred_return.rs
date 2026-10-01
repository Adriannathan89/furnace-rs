//! Confirms provider attributes require a concrete return type.

#[mads::element]
fn value() -> _ {
    String::new()
}

fn main() {}
