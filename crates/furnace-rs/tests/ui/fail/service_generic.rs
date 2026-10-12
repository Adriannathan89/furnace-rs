//! Confirms generic services receive a focused diagnostic.

#[furnace_rs::burner]
struct GenericService<T> {
    #[inject]
    value: T,
}

fn main() {}
