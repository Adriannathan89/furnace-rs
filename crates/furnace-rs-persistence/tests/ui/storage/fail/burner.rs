//! Compile-time entity storage contract.
#![allow(dead_code, unused_imports)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path = "../../../support/storage_entities.rs"]
mod entities;
#[furnace_rs_core::burner(entity = entities::post::Entity)]
struct Service { #[inject] database: DatabaseConnection }
fn main() {}
