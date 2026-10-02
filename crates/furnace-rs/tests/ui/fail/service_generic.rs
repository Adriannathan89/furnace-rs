//! Confirms generic services receive a focused diagnostic.

#[furnace_rs::burner]
struct GenericService<T> {
    value: T,
}

fn main() {}
