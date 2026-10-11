use furnace_rs::{burner, cauldron, Logger, LoggerCauldron};
use furnace_rs_persistence::sea_orm::{DatabaseCauldron, DatabaseConnection};
#[burner] struct Consumer { #[inject] database: DatabaseConnection, logger: Logger }
#[cauldron(imports=[DatabaseCauldron, LoggerCauldron])] struct Left;
#[cauldron(imports=[DatabaseCauldron, LoggerCauldron])] struct Right;
#[cauldron(root, imports=[Left, Right, DatabaseCauldron, LoggerCauldron], providers=[Consumer])] struct App;
fn main() {}
