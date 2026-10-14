use furnace_rs_core::{burner, cauldron};
#[burner] struct Controller;
#[cauldron(controllers=[Controller], exports=[Controller])] struct Module;
fn main() {}
