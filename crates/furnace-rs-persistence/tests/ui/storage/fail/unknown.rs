//! Compile-time entity storage contract.
#![allow(dead_code, unused_imports)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path = "../../../support/storage_entities.rs"]
mod entities;
#[storage(entity = entities::post::Entity, pagination(size = 20))]
struct Repo { #[inject] database: DatabaseConnection }
fn main() {}
