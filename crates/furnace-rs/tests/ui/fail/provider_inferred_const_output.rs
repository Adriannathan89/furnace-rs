//! Confirms provider attributes reject inferred output array lengths.

#[furnace_rs::element]
fn value() -> [u8; _] {
    [0; 4]
}

fn main() {}
