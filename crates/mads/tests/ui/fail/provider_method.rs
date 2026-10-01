//! Confirms provider attributes reject inherent methods.

struct Factory;

impl Factory {
    #[mads::element]
    fn value(&self) -> String {
        String::new()
    }
}

fn main() {}
