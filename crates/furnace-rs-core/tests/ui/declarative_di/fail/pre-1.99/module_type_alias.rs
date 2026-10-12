use furnace_rs_core::cauldron;
#[cauldron(providers=[])] struct Module;
type Alias = Module;
#[cauldron(root, imports=[Alias])] struct App;
fn main() {}
