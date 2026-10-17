//! Relation loader compiler contract.
#![allow(dead_code, unused_imports)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path="../../../support/storage_relations.rs"] mod entities;
use entities::{post,user,comment};
#[storage(entity=post::Entity,relations(author=post::Relation::Author,author=post::Relation::Editor))]
struct Repo { #[inject] database:DatabaseConnection }
fn main() {}
