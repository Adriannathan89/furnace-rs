//! Confirms provider attributes reject lifetime-generic functions.

#[furnace_rs::element]
fn value<'value>() -> String {
    String::new()
}

fn main() {}
