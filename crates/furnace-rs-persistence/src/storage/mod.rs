//! Typed entity operations and explicit pagination over native SeaORM.

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
