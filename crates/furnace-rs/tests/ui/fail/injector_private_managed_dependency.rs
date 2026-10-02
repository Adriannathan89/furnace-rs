#![allow(dead_code)]

#[derive(Clone)]
struct PrivateDependency;

#[furnace_rs::burner]
pub struct PublicService {
    dependency: PrivateDependency,
}

#[furnace_rs::storage]
pub struct PublicRepository {
    dependency: PrivateDependency,
}

#[furnace_rs::controller]
pub struct PublicController {
    dependency: PrivateDependency,
}

fn main() {}
