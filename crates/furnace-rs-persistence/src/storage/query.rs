//! Native query forwarding with explicit order and checked page bounds.
use super::{Page, PageWindow, StorageError, StorageErrorKind, StorageResult};
use sea_orm::{
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Select,
    sea_query::{IntoCondition, SelectStatement},
};

/// A root entity query borrowing the repository connection.
pub struct EntityQuery<'db, E: EntityTrait> {
    database: &'db DatabaseConnection,
    select: Select<E>,
    ordered: bool,
}

impl<'db, E: EntityTrait> EntityQuery<'db, E> {
    pub(super) fn new(database: &'db DatabaseConnection) -> Self {
        Self {
            database,
            select: E::find(),
            ordered: false,
        }
    }

    /// Adds a native predicate, combining repeated predicates with AND.
    pub fn filter<C: IntoCondition>(mut self, condition: C) -> Self {
        self.select = self.select.filter(condition);

        self
    }

    /// Appends an explicit ascending root-column order.
    pub fn order_by_asc(mut self, column: E::Column) -> Self {
        self.select = self.select.order_by_asc(column);
        self.ordered = true;

        self
    }

    /// Appends an explicit descending root-column order.
    pub fn order_by_desc(mut self, column: E::Column) -> Self {
        self.select = self.select.order_by_desc(column);
        self.ordered = true;

        self
    }

    /// Returns one matching row; unordered selection is intentionally unspecified.
    pub async fn one(self) -> StorageResult<Option<E::Model>> {
        self.select.one(self.database).await.map_err(Into::into)
    }

    /// Reads exactly the requested one-based page without a count or lookahead.
    ///
    /// At least one explicit order is required. Size limits are application policy.
    pub async fn paginate(self, page_number: u64, page_size: u64) -> StorageResult<Page<E::Model>> {
        let PreparedPage {
            database,
            select,
            window,
        } = self.prepare_page(page_number, page_size)?;
        let items = select.all(database).await?;

        Ok(window.finish(items))
    }

    #[doc(hidden)]
    pub fn prepare_page(
        self,
        page_number: u64,
        page_size: u64,
    ) -> StorageResult<PreparedPage<'db, E>> {
        let window = PageWindow::new(page_number, page_size)?;

        if !self.ordered {
            return Err(StorageError::__new(StorageErrorKind::MissingOrder));
        }

        Ok(PreparedPage {
            database: self.database,
            select: self
                .select
                .offset(window.offset())
                .limit(window.fetch_limit()),
            window,
        })
    }

    #[doc(hidden)]
    pub fn prepare_one(self) -> (&'db DatabaseConnection, SelectStatement) {
        (self.database, self.select.limit(1).into_query())
    }
}

#[doc(hidden)]
pub struct PreparedPage<'db, E: EntityTrait> {
    database: &'db DatabaseConnection,
    select: Select<E>,
    window: PageWindow,
}

impl<'db, E: EntityTrait> PreparedPage<'db, E> {
    /// Borrows the connection used by this prepared query.
    pub fn database(&self) -> &'db DatabaseConnection {
        self.database
    }

    /// Copies the bounded statement for a native entity loader.
    pub fn statement(&self) -> SelectStatement {
        self.select.clone().into_query()
    }

    /// Wraps native root or relation-aware results without a second allocation.
    pub fn finish<T>(self, items: Vec<T>) -> Page<T> {
        self.window.finish(items)
    }
}
