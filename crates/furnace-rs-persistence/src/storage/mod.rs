//! Typed entity operations and explicit pagination over native SeaORM.
//!
//! Enable `sea-orm-storage` and select a database driver separately. Generated
//! helpers borrow the injected connection; applications own transactions and
//! pagination limits. Collection reads require explicit ordering and issue no
//! count or lookahead query.
//!
//! # Basic entity storage
//!
//! ```
//! use furnace_rs_core::{storage, Injector};
//! use furnace_rs_persistence::storage::StorageResult;
//! use sea_orm::{DatabaseConnection, ColumnTrait, Set};
//! # mod post {
//! # use sea_orm::entity::prelude::*;
//! # #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! # #[sea_orm(table_name="posts")]
//! # pub struct Model { #[sea_orm(primary_key)] pub id:i32, pub title:String }
//! # #[derive(Copy,Clone,Debug,EnumIter,DeriveRelation)] pub enum Relation {}
//! # impl ActiveModelBehavior for ActiveModel {}
//! # }
//!
//! #[storage(entity = post::Entity)]
//! struct Posts {
//!     #[inject]
//!     database: DatabaseConnection,
//! }
//!
//! # async fn example(database: DatabaseConnection) -> StorageResult<()> {
//! # let repository = Posts::inject((database,)).await.expect("injected connection");
//! let inserted = repository.entity().insert(post::ActiveModel {
//!     title: Set("Rust".into()),
//!     ..Default::default()
//! }).await?;
//! let page = repository.entity().query()
//!     .filter(post::Column::Title.contains("Rust"))
//!     .order_by_desc(post::Column::Id)
//!     .paginate(1, 20)
//!     .await?;
//! assert_eq!(page.page_number, 1);
//! # let _ = inserted;
//! # Ok(())
//! # }
//! ```
//!
//! # Native relation results
//!
//! ```
//! # use furnace_rs_core::{storage,Injector};
//! # use furnace_rs_persistence::storage::StorageResult;
//! # use sea_orm::DatabaseConnection;
//! # mod user {
//! # use sea_orm::entity::prelude::*;
//! # #[sea_orm::model]
//! # #[derive(Clone,Debug,PartialEq,DeriveEntityModel)]
//! # #[sea_orm(table_name="users")]
//! # pub struct Model { #[sea_orm(primary_key)] pub id:i32, pub name:String }
//! # impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # mod post {
//! # use sea_orm::entity::prelude::*;
//! # #[sea_orm::model]
//! # #[derive(Clone,Debug,PartialEq,DeriveEntityModel)]
//! # #[sea_orm(table_name="posts")]
//! # pub struct Model {
//! # #[sea_orm(primary_key)] pub id:i32, pub user_id:i32,
//! # #[sea_orm(belongs_to,from="user_id",to="id")]
//! # pub author:BelongsTo<super::user::Entity>,
//! # }
//! # impl ActiveModelBehavior for ActiveModel {}
//! # }
//! #[storage(entity = post::Entity, relations(author = user::Entity))]
//! struct Posts {
//!     #[inject]
//!     database: DatabaseConnection,
//! }
//! # async fn example(database:DatabaseConnection)->StorageResult<()> {
//! # let repository=Posts::inject((database,)).await.expect("injected connection");
//! let page = repository.entity().query()
//!     .with_author()
//!     .order_by_asc(post::Column::Id)
//!     .paginate(1, 20)
//!     .await?;
//! // page.items contains native post::ModelEx, including its author wrapper.
//! # let _: Vec<post::ModelEx> = page.items;
//! # Ok(())
//! # }
//! # fn main() {}
//! ```

mod entity;
mod error;
mod pagination;
mod query;

#[doc(hidden)]
pub use ::sea_orm as __sea_orm;
pub use entity::EntityStore;
pub use error::{StorageError, StorageErrorKind, StorageResult};
pub use pagination::Page;
#[doc(hidden)]
pub use pagination::PageWindow;
pub use query::EntityQuery;
#[doc(hidden)]
pub use query::PreparedPage;
