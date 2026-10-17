mod auth;

use auth::AuthCauldron;
use furnace_rs::prelude::*;

#[cauldron(root, imports = [AuthCauldron])]
struct AppCauldron;

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
