use furnace_rs_core::{burner, cauldron};
#[burner] struct Value;
#[cauldron(providers=[Value], exports=[Value])] struct Infrastructure;
#[cauldron(imports=[Infrastructure])] struct Parent;
#[burner] struct Consumer { value: Value }
#[cauldron(root, imports=[Parent], providers=[Consumer])] struct App;
fn main() {}
