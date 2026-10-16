//! Relation loader compiler contract.
#![deny(missing_docs)]
#![allow(dead_code, unused_imports)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path="../../../support/storage_relations.rs"] mod entities;
use entities::{post,user,comment};

#[storage(entity=post::Entity,relations(author=post::Relation::Author,comments=comment::Entity,r#type=comment::Entity))]
struct Repo { #[inject] database:DatabaseConnection }
#[storage(entity=post::Entity,relations(editor=post::Relation::Editor))]
struct Other { #[inject] database:DatabaseConnection }
mod a {
    use super::*;
    /// A storage sharing its final name with a different module.
    #[storage(entity=post::Entity,relations(comments=comment::Entity))]
    pub struct Same { #[inject] database:DatabaseConnection }
}
mod b {
    use super::*;
    /// A storage sharing its final name with a different module.
    #[storage(entity=post::Entity,relations(comments=comment::Entity))]
    pub struct Same { #[inject] database:DatabaseConnection }
}
async fn exercise(repo:&Repo) {
    let query=repo.entity().query().with_author().with_type().order_by_asc(post::Column::Id);
    let _=query.paginate(1,20).await;
}
fn main() {}
