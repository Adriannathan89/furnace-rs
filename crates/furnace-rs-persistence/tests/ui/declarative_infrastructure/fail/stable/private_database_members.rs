use furnace_rs::{burner, cauldron};
use furnace_rs_persistence::{DatabaseFactory, sea_orm::{DatabaseCauldron, SeaOrmPostgres}};
#[burner] struct Consumer { factory: DatabaseFactory, connector: SeaOrmPostgres }
#[cauldron(root, imports=[DatabaseCauldron], providers=[Consumer])] struct App;
fn main() {}
