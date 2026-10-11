use furnace_rs_core::{burner, cauldron};
#[burner] struct A;
#[burner] struct B { a: A }
#[cauldron(root, providers=[A, B], exports=[A])]
struct App;
fn main() {}
