//! Confirms provider attributes reject unsafe functions.

#[mads::element]
unsafe fn value() -> String {
    String::new()
}

fn main() {}
