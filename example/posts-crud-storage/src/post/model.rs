use furnace_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
}

impl From<entity::Model> for Post {
    fn from(model: entity::Model) -> Self {
        Self {
            id: model.id,
            title: model.title,
            body: model.body,
        }
    }
}

#[derive(Deserialize, Input)]
pub struct PostInput {
    #[validate(nonempty, length(max = 120))]
    pub title: String,
    #[validate(nonempty)]
    pub body: String,
}

#[derive(Deserialize)]
pub struct PostPagination {
    pub page_number: u64,
    pub page_size: u64,
}

pub mod entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub title: String,
        pub body: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
