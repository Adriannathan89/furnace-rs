//! Confirms controller dependency constraints point at the declared field type.

// Keep the Clone suggestion on a two-digit line so stable and Rust 1.94
// render its help gutter identically.
//
//
//
//
//
struct NotClone;

#[furnace_rs::controller]
struct Controller {
    dependency: NotClone,
}

impl furnace_rs::Sealable for Controller {
    fn seals() -> furnace_rs::SealRegistration<Self> { furnace_rs::SealRegistration::new() }
}

fn main() {}
