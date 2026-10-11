use furnace_rs_core::{burner, cauldron};
#[burner] struct Hidden;
#[cauldron(providers=[Hidden], global)] struct Infrastructure;
#[burner] struct Consumer { hidden: Hidden }
#[cauldron(root, imports=[Infrastructure], providers=[Consumer])] struct App;
fn main() {}
