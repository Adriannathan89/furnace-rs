use furnace_rs::{burner, cauldron, Logger, LoggerCauldron};
use furnace_rs_persistence::sea_orm::{DatabaseCauldron, DatabaseConnection};
#[burner] struct Consumer { #[inject] database: DatabaseConnection, #[inject] logger: Logger }
#[cauldron(root, imports=[LoggerCauldron], providers=[Consumer])] struct App;
fn main() {}
