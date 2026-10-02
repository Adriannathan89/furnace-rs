//! Verifies main expansion through a facade-only dependency.

use furnace_rs::prelude::*;

#[storage]
struct MainRepository {
    value: u32,
}

fn consume_repository(repository: &MainRepository) {
    let _ = repository.value;
}

#[furnace_rs::main]
async fn main() {
    let _ = consume_repository;
}
