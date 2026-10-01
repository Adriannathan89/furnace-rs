//! Confirms managed-provider fields reject non-documentation attributes.

#[mads::burner]
struct AttributedFieldService {
    #[allow(dead_code)]
    dependency: String,
}

fn main() {}
