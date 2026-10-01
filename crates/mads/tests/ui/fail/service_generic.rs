//! Confirms generic services receive a focused diagnostic.

#[mads::burner]
struct GenericService<T> {
    value: T,
}

fn main() {}
