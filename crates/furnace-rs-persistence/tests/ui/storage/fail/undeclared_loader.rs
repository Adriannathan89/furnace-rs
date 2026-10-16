//! Relation loader compiler contract.
#![allow(dead_code, unused_imports)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path="../../../support/storage_relations.rs"] mod entities;
use entities::{post,user,comment};
#[storage(entity=post::Entity,relations(author=post::Relation::Author))]
struct Repo { #[inject] database:DatabaseConnection }
fn exercise(repo:&Repo) {let _=repo.entity().query().with_secret();}
fn main() {}
