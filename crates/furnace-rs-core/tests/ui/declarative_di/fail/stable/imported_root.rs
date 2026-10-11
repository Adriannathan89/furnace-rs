use furnace_rs_core::cauldron;
#[cauldron(root)] struct First;
#[cauldron(root, imports=[First])] struct Second;
fn main() {}
