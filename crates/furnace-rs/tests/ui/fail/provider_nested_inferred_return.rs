//! Confirms provider attributes reject inferred types nested in result output.

#[furnace_rs::element]
fn value() -> furnace_rs::core::Result<_> {
    Ok(String::new())
}

fn main() {}
