//! Compile-time entity storage contract.
#![allow(dead_code, unused_imports)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path = "../../../support/storage_entities.rs"]
mod entities;
type Db = DatabaseConnection;
#[storage(entity = entities::post::Entity)]
struct Repo { #[inject] database: Db }
fn main() {}
