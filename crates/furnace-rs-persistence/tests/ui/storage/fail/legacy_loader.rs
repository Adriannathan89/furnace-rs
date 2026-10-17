//! Legacy entities cannot produce native relation loaders.
#![allow(dead_code)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path="../../../support/storage_entities.rs"] mod entities;
#[storage(entity=entities::post::Entity,relations(other=entities::composite::Entity))]
struct Repo { #[inject] database:DatabaseConnection }
fn main() {}
