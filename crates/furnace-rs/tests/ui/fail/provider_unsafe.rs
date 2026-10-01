//! Confirms provider attributes reject unsafe functions.

#[furnace_rs::element]
unsafe fn value() -> String {
    String::new()
}

fn main() {}
