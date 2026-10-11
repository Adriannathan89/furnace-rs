use furnace_rs::{burner, cauldron, Logger, LoggerCauldron};
use furnace_rs_persistence::sea_orm::{DatabaseCauldron, DatabaseConnection};
#[burner] struct Consumer { #[inject] database: DatabaseConnection, logger: Logger }
#[cauldron(root, imports=[DatabaseCauldron], providers=[Consumer])] struct App;
fn main() {}
