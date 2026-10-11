#![allow(non_camel_case_types)]
use furnace_rs_core::{burner, cauldron};
#[burner] struct Value;
type Alias = Value;
#[cauldron(providers=[Value], exports=[Alias])]
struct Module;
#[burner] struct Consumer { value: Alias }
#[cauldron(root, imports=[Module], providers=[Consumer])]
struct r#type;
fn main() {}
