mod post;

use furnace_rs::prelude::*;
use furnace_rs_persistence::sea_orm::DatabaseCauldron;
use post::PostCauldron;

#[cauldron(root, imports = [DatabaseCauldron, PostCauldron])]
struct AppCauldron;

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
