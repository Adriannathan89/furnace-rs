//! Compile-time entity storage contract.
#![deny(missing_docs)]
#![allow(dead_code, unused_imports)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
#[path = "../../../support/storage_entities.rs"]
mod entities;

/// A managed repository.
#[storage(entity = entities::post::Entity)]
pub struct Repo { #[inject] database: DatabaseConnection }
#[storage(entity = entities::post::Entity)]
struct Qualified { #[inject] database: sea_orm::DatabaseConnection }
type Db = DatabaseConnection;
#[furnace_rs::storage(entity = entities::post::Entity, connection = primary)]
struct Alias { #[inject] primary: Db, #[inject] replica: DatabaseConnection }
async fn query(repo: &Repo) {
    let query = repo.entity().query().order_by_desc(entities::post::Column::Id);
    let _ = query.paginate(1,20).await;
    let _ = repo.entity().find_by_id(1).await;
}
fn main() {}
