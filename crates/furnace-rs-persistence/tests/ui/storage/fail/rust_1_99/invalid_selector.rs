//! Relation loader compiler contract.
#![allow(dead_code, unused_imports)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path="../../../../support/storage_relations.rs"] mod entities;
use entities::{post,user,comment};
#[storage(entity=post::Entity,relations(author=user::Entity))]
struct Repo { #[inject] database:DatabaseConnection }
fn main() {}
