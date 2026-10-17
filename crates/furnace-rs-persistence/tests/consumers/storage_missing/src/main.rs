//! Independent storage macro consumer.
#![deny(missing_docs)]
#![allow(dead_code)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path = "../../../support/storage_relations.rs"]
mod entities;
#[storage(entity = entities::post::Entity, relations(comments = entities::comment::Entity))]
struct Repo { #[inject] database: DatabaseConnection }
fn main() {}
