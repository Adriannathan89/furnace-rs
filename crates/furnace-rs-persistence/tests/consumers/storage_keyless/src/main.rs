//! Independent rejection of entities without a primary key.
#![allow(dead_code)]
use furnace_rs_core::storage;
use sea_orm::DatabaseConnection;
mod keyless {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "keyless")]
    pub struct Model { pub title: String }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
#[storage(entity = keyless::Entity)]
struct Repo { #[inject] database: DatabaseConnection }
fn main() {}
