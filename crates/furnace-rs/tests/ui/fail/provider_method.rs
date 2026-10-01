//! Confirms provider attributes reject inherent methods.

struct Factory;

impl Factory {
    #[furnace_rs::element]
    fn value(&self) -> String {
        String::new()
    }
}

fn main() {}
