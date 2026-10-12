use furnace_rs_core::{burner, cauldron};
#[burner] struct Consumer { #[inject] automatic: u32 }
#[cauldron(root, providers=[Consumer])] struct App;
fn main() {}
