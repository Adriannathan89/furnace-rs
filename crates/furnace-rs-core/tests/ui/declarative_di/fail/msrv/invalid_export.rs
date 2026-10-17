use furnace_rs_core::{burner, cauldron};
#[burner] struct Value;
#[cauldron(exports=[Value])] struct Module;
fn main() {}
