//! Pagination arithmetic, result envelopes, and redacted storage failures.
#![cfg(feature = "sea-orm-storage")]

use furnace_rs_persistence::storage::{Page, PageWindow, StorageError, StorageErrorKind};
use std::error::Error;

#[test]
fn validates_offsets_without_caps_or_lookahead() {
    for (number, size, offset) in [
        (1, 20, 0),
        (2, 20, 20),
        (2, 1_000_000, 1_000_000),
        (1, u64::MAX, 0),
        (2, u64::MAX, u64::MAX),
    ] {
        let window = PageWindow::new(number, size).unwrap();
        assert_eq!(
            (
                window.page_number(),
                window.page_size(),
                window.offset(),
                window.fetch_limit()
            ),
            (number, size, offset, size)
        );
    }
    for (number, size) in [(0, 20), (1, 0), (3, u64::MAX)] {
        assert_eq!(
            PageWindow::new(number, size).unwrap_err().kind(),
            StorageErrorKind::InvalidPagination
        );
    }
}

#[test]
fn envelopes_preserve_items_and_only_requested_pagination_metadata() {
    let page = PageWindow::new(2, 20).unwrap().finish(vec![1, 2]);
    assert_eq!(
        serde_json::to_value(page).unwrap(),
        serde_json::json!({"items":[1,2],"page_number":2,"page_size":20})
    );
    let empty = PageWindow::new(99, 20).unwrap().finish(Vec::<u8>::new());
    assert_eq!(
        (empty.items.len(), empty.page_number, empty.page_size),
        (0, 99, 20)
    );
    struct NonSerializable;
    let _: Page<NonSerializable> = PageWindow::new(1, 1).unwrap().finish(vec![NonSerializable]);
}

#[test]
fn database_error_source_is_retained_but_redacted() {
    let error = StorageError::from(sea_orm::DbErr::Custom("secret SQL parameter".into()));
    assert_eq!(error.kind(), StorageErrorKind::Database);
    assert!(!format!("{error} {error:?}").contains("secret"));
    assert!(
        error
            .source()
            .unwrap()
            .downcast_ref::<sea_orm::DbErr>()
            .is_some()
    );
}
