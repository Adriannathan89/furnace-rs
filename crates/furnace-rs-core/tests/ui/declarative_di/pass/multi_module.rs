use furnace_rs_core::{burner, cauldron};
#[burner] struct Value;
#[cauldron(providers=[Value], exports=[Value], global)] struct Base;
#[cauldron(imports=[Base])] struct Left0;
#[cauldron(imports=[Base])] struct Right0;
#[cauldron(imports=[Left0, Right0])] struct Level0;
#[cauldron(imports=[Level0])] struct Left1;
#[cauldron(imports=[Level0])] struct Right1;
#[cauldron(imports=[Left1, Right1])] struct Level1;
#[cauldron(imports=[Level1])] struct Left2;
#[cauldron(imports=[Level1])] struct Right2;
#[cauldron(imports=[Left2, Right2])] struct Level2;
#[cauldron(imports=[Level2])] struct Left3;
#[cauldron(imports=[Level2])] struct Right3;
#[cauldron(imports=[Left3, Right3])] struct Level3;
#[cauldron(imports=[Level3])] struct Left4;
#[cauldron(imports=[Level3])] struct Right4;
#[cauldron(imports=[Left4, Right4])] struct Level4;
#[cauldron(imports=[Level4])] struct Left5;
#[cauldron(imports=[Level4])] struct Right5;
#[cauldron(imports=[Left5, Right5])] struct Level5;
#[burner] struct Consumer { #[inject] value: Value }
#[cauldron(root, imports=[Level5], providers=[Consumer])] struct App;
fn main() {}
