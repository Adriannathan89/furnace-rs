//! Confirms managed dependency constraints point at the declared field type.

// Keep both Clone suggestions on two-digit lines so stable and Rust 1.94
// render their help gutters identically.
//
//
//
//
//
//
struct NotCloneProvider;

struct ProviderOutput;

#[allow(dead_code)]
#[furnace_rs::element]
fn provide(_dependency: NotCloneProvider) -> ProviderOutput {
    ProviderOutput
}

struct NotCloneManaged;

#[furnace_rs::burner]
struct Service {
    dependency: NotCloneManaged,
}

fn main() {}
