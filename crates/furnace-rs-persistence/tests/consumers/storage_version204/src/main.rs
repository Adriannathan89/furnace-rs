//! Independent storage macro consumer.
#![deny(missing_docs)]
#![allow(dead_code)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path = "../../../support/storage_relations.rs"]
mod entities;
#[storage(entity = entities::post::Entity, relations(comments = entities::comment::Entity))]
struct Repo { #[inject] database: DatabaseConnection }
async fn example(repo: &Repo) {
    let query = repo.entity().query().with_comments().order_by_asc(entities::post::Column::Id);
    let _ = query.paginate(1,20).await;
}
fn main() {}
