mod post;

use furnace_rs::prelude::*;
use furnace_rs_persistence::sea_orm::DatabaseCauldron;
use post::PostCauldron;

#[cauldron]
struct AppCauldron;

impl furnace_rs::core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.import(DatabaseCauldron).import(PostCauldron)
    }
}

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
