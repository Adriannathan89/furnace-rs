//! Confirms managed providers accept dependencies with the generated contract.

#[derive(Clone)]
struct Dependency;

#[furnace_rs::burner]
struct Service {
    #[inject]
    dependency: Dependency,
}

fn main() {}
