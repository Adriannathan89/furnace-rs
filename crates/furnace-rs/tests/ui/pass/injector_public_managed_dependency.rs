#![allow(dead_code)]

#[derive(Clone)]
pub struct PublicDependency;

#[furnace_rs::burner]
pub struct PublicService {
    dependency: PublicDependency,
}

#[furnace_rs::storage]
pub struct PublicRepository {
    dependency: PublicDependency,
}

#[furnace_rs::controller]
pub struct PublicController {
    dependency: PublicDependency,
}

fn assert_injector<T: furnace_rs::Injector<Dependencies = (PublicDependency,)>>() {}

fn main() {
    assert_injector::<PublicService>();
    assert_injector::<PublicRepository>();
    assert_injector::<PublicController>();
}
