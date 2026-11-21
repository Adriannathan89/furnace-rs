mod accounts;

use accounts::AccountsCauldron;
use furnace_rs::prelude::*;

#[cauldron(root, imports = [AccountsCauldron])]
struct AppCauldron;

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
