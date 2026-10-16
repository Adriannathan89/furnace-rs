//! Relation storage also requires an exact native connection.
#![allow(dead_code)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path="../../../support/storage_relations.rs"] mod entities;
#[storage(entity=entities::post::Entity,connection=database,relations(comments=entities::comment::Entity))]
struct Repo { #[inject] database:std::sync::Arc<DatabaseConnection> }
fn main() {}
