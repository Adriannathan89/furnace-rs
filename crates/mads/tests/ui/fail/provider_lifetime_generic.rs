//! Confirms provider attributes reject lifetime-generic functions.

#[mads::element]
fn value<'value>() -> String {
    String::new()
}

fn main() {}
