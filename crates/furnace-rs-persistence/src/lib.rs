//! Native persistence connector contracts for furnace-rs applications.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod error;
mod factory;
#[cfg(feature = "sea-orm-postgres")]
pub mod sea_orm;

/// Optional entity-bound SeaORM repository operations.
#[cfg(feature = "sea-orm-storage")]
pub mod storage;

#[doc(hidden)]
#[cfg(feature = "sea-orm-storage")]
#[macro_export]
macro_rules! __furnace_storage_enabled {
    ($($items:tt)*) => { $($items)* };
}

#[doc(hidden)]
#[cfg(not(feature = "sea-orm-storage"))]
#[macro_export]
macro_rules! __furnace_storage_enabled {
    ($($items:tt)*) => {
        compile_error!("entity-bound storage requires the `sea-orm-storage` feature on `furnace-rs-persistence`");
    };
}

pub use error::{FURNACE140, PersistenceError, PersistenceErrorKind, PersistenceResult};
pub use factory::{DatabaseConnector, DatabaseFactory};
