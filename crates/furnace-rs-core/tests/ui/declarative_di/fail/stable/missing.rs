use furnace_rs_core::{burner, cauldron};
#[burner]
struct Missing;
#[burner]
struct Consumer { #[inject] missing: Missing }
#[cauldron(root, providers = [Consumer])]
struct App;
fn main() {}
