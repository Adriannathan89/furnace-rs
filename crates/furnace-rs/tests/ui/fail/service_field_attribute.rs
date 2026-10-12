//! Confirms managed-provider fields reject non-documentation attributes.

#[furnace_rs::burner]
struct AttributedFieldService {
    #[inject]
    #[allow(dead_code)]
    dependency: String,
}

fn main() {}
