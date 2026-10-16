//! Compile-time entity storage contract.
#![allow(dead_code, unused_imports)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path = "../../../support/storage_entities.rs"]
mod entities;
#[storage(entity = entities::post::Entity)]
struct Repo { database: DatabaseConnection }
fn main() {}
