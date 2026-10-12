#![allow(dead_code)]

#[derive(Clone)]
struct PrivateDependency;

#[furnace_rs::burner]
pub struct PublicService {
    #[inject]
    dependency: PrivateDependency,
}

#[furnace_rs::storage]
pub struct PublicRepository {
    #[inject]
    dependency: PrivateDependency,
}

#[furnace_rs::controller]
pub struct PublicController {
    #[inject]
    dependency: PrivateDependency,
}

fn main() {}
