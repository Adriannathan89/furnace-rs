#[furnace_rs::controller]
struct Controller<T> {
    #[inject]
    value: T,
}

fn main() {}
