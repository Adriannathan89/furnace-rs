//! A similarly named type is not a SeaORM connection.
use furnace_rs_core::storage;
#[path = "../../../support/storage_entities.rs"] mod entities;
#[derive(Clone)] struct DatabaseConnection;
#[storage(entity = entities::post::Entity)]
struct Repo { #[inject] database: DatabaseConnection }
fn main() {}
