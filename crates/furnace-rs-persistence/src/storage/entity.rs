//! Borrowed, statically typed access to native SeaORM entity operations.

use super::{StorageError, StorageErrorKind, StorageResult};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, PrimaryKeyTrait,
};
use std::marker::PhantomData;

/// Basic entity operations borrowing an application's existing connection.
pub struct EntityStore<'db, E: EntityTrait> {
    pub(super) database: &'db DatabaseConnection,
    entity: PhantomData<E>,
}

impl<'db, E: EntityTrait> EntityStore<'db, E> {
    /// Builds a native entity query borrowing the connection, not this helper.
    pub fn query(&self) -> super::EntityQuery<'db, E> {
        super::EntityQuery::new(self.database)
    }
    /// Borrows a connection without opening, cloning, or executing anything.
    pub fn new(database: &'db DatabaseConnection) -> Self {
        Self {
            database,
            entity: PhantomData,
        }
    }

    /// Finds a record by its native single or composite primary key.
    pub async fn find_by_id<K>(&self, key: K) -> StorageResult<Option<E::Model>>
    where
        K: Into<<E::PrimaryKey as PrimaryKeyTrait>::ValueType>,
    {
        E::find_by_id(key)
            .one(self.database)
            .await
            .map_err(Into::into)
    }

    /// Inserts the selected fields and preserves native ActiveModel hooks.
    pub async fn insert(&self, model: E::ActiveModel) -> StorageResult<E::Model>
    where
        E::Model: IntoActiveModel<E::ActiveModel>,
        E::ActiveModel: Send,
    {
        model.insert(self.database).await.map_err(Into::into)
    }

    /// Updates selected fields; an absent record is a typed NotFound error.
    pub async fn update(&self, model: E::ActiveModel) -> StorageResult<E::Model>
    where
        E::Model: IntoActiveModel<E::ActiveModel>,
        E::ActiveModel: Send,
    {
        model.update(self.database).await.map_err(|error| {
            if matches!(error, sea_orm::DbErr::RecordNotUpdated) {
                StorageError::__with_source(StorageErrorKind::NotFound, error)
            } else {
                error.into()
            }
        })
    }

    /// Deletes by primary key without instantiating a model or invoking its hooks.
    pub async fn delete_by_id<K>(&self, key: K) -> StorageResult<bool>
    where
        K: Into<<E::PrimaryKey as PrimaryKeyTrait>::ValueType>,
    {
        Ok(E::delete_by_id(key)
            .exec(self.database)
            .await?
            .rows_affected
            != 0)
    }
}
