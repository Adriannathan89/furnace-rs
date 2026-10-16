//! Native relation-aware entity fixtures.
#![allow(dead_code, missing_docs)]
pub mod user {
    use sea_orm::entity::prelude::*;
    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "storage_users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
    }
    impl ActiveModelBehavior for ActiveModel {}
}
pub mod post {
    use sea_orm::entity::prelude::*;
    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "storage_loaded_posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub title: String,
        pub author_id: i32,
        pub editor_id: i32,
        #[sea_orm(belongs_to, from = "author_id", to = "id", relation_enum = "Author")]
        pub author: BelongsTo<super::user::Entity>,
        #[sea_orm(belongs_to, from = "editor_id", to = "id", relation_enum = "Editor")]
        pub editor: BelongsTo<super::user::Entity>,
        #[sea_orm(has_many)]
        pub comments: HasMany<super::comment::Entity>,
        #[sea_orm(has_many, via = "post_tag")]
        pub tags: HasMany<super::tag::Entity>,
    }
    impl ActiveModelBehavior for ActiveModel {}
}
pub mod comment {
    use sea_orm::entity::prelude::*;
    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "storage_comments")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub post_id: i32,
        pub body: String,
        #[sea_orm(belongs_to, from = "post_id", to = "id")]
        pub post: BelongsTo<super::post::Entity>,
    }
    impl ActiveModelBehavior for ActiveModel {}
}
pub mod tag {
    use sea_orm::entity::prelude::*;
    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "storage_tags")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        #[sea_orm(has_many, via = "post_tag")]
        pub posts: HasMany<super::post::Entity>,
    }
    impl ActiveModelBehavior for ActiveModel {}
}
pub mod post_tag {
    use sea_orm::entity::prelude::*;
    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "storage_post_tags")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub post_id: i32,
        #[sea_orm(primary_key, auto_increment = false)]
        pub tag_id: i32,
        #[sea_orm(belongs_to, from = "post_id", to = "id")]
        pub post: BelongsTo<super::post::Entity>,
        #[sea_orm(belongs_to, from = "tag_id", to = "id")]
        pub tag: BelongsTo<super::tag::Entity>,
    }
    impl ActiveModelBehavior for ActiveModel {}
}
