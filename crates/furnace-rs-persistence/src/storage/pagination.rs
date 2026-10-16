//! Page envelopes and checked one-based offset arithmetic.

use super::{StorageError, StorageErrorKind, StorageResult};

/// A requested page of parent records, without lookahead or a total count.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Page<T> {
    /// Records returned for this page.
    pub items: Vec<T>,
    /// Requested one-based page number, including for an empty page.
    pub page_number: u64,
    /// Requested page size, without framework limits or clamping.
    pub page_size: u64,
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct PageWindow {
    page_number: u64,
    page_size: u64,
    offset: u64,
}

impl PageWindow {
    /// Validates positive arguments and checks the calculated offset.
    pub fn new(page_number: u64, page_size: u64) -> StorageResult<Self> {
        let offset = page_number
            .checked_sub(1)
            .filter(|_| page_size != 0)
            .and_then(|page| page.checked_mul(page_size))
            .ok_or_else(|| StorageError::new(StorageErrorKind::InvalidPagination))?;
        Ok(Self {
            page_number,
            page_size,
            offset,
        })
    }

    /// Returns the requested one-based page number.
    pub const fn page_number(&self) -> u64 {
        self.page_number
    }
    /// Returns the requested page size.
    pub const fn page_size(&self) -> u64 {
        self.page_size
    }
    /// Returns the checked zero-based row offset.
    pub const fn offset(&self) -> u64 {
        self.offset
    }
    /// Returns the SQL limit without fetching a lookahead record.
    pub const fn fetch_limit(&self) -> u64 {
        self.page_size
    }
    /// Wraps returned rows without allocating or trimming another page.
    pub fn finish<T>(self, items: Vec<T>) -> Page<T> {
        Page {
            items,
            page_number: self.page_number,
            page_size: self.page_size,
        }
    }
}
