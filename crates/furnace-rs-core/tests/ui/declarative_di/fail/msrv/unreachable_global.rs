use furnace_rs_core::{burner, cauldron};
#[burner] struct Value;
#[cauldron(providers=[Value], exports=[Value], global)] struct Infrastructure;
#[burner] struct Consumer { value: Value }
#[cauldron(root, providers=[Consumer])] struct App;
fn main() {}
