use furnace_rs_core::{burner, cauldron};
#[burner]
struct Missing;
#[burner]
struct Consumer { missing: Missing }
#[cauldron(root, providers = [Consumer])]
struct App;
fn main() {}
