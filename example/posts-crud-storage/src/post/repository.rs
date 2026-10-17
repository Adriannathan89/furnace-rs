use furnace_rs::prelude::*;
use furnace_rs_persistence::sea_orm::DatabaseConnection;

use super::model::entity;

#[storage(entity = entity::Entity)]
pub struct PostRepository {
    #[inject]
    database: DatabaseConnection,
}
