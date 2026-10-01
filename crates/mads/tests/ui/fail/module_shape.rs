//! Confirms non-unit modules receive a focused diagnostic.

#[mads::furnace]
struct InvalidModule {
    enabled: bool,
}

fn main() {}
