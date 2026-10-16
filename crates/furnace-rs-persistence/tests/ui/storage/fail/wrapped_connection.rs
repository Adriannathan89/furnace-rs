//! Wrapped connections are outside the native connection contract.
#![allow(dead_code)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path="../../../support/storage_entities.rs"] mod entities;
#[storage(entity=entities::post::Entity,connection=database)]
struct Repo { #[inject] database:std::sync::Arc<DatabaseConnection> }
fn main() {}
