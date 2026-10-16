//! Shared native entity fixtures for storage integration tests.
#![allow(dead_code, missing_docs)]

pub mod hooked {
    use sea_orm::entity::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    pub static BEFORE: AtomicUsize = AtomicUsize::new(0);
    pub static AFTER: AtomicUsize = AtomicUsize::new(0);
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "storage_hooked")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub title: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    #[async_trait::async_trait]
    impl ActiveModelBehavior for ActiveModel {
        async fn before_save<C: ConnectionTrait>(mut self, _: &C, _: bool) -> Result<Self, DbErr> {
            BEFORE.fetch_add(1, Ordering::SeqCst);
            self.title = sea_orm::Set("before".into());
            Ok(self)
        }
        async fn after_save<C: ConnectionTrait>(
            model: Model,
            _: &C,
            _: bool,
        ) -> Result<Model, DbErr> {
            AFTER.fetch_add(1, Ordering::SeqCst);
            Ok(model)
        }
    }
}

pub mod post {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "storage_posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub title: String,
        pub published: bool,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod composite {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "storage_composite")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub tenant_id: i32,
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: i32,
        pub title: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
