//! Typed queries preserve caller ordering and issue only the requested page.
#![cfg(feature = "sea-orm-storage")]
#[path = "support/storage_entities.rs"]
mod entities;
use entities::{composite, post};
use furnace_rs_persistence::storage::{EntityStore, StorageErrorKind};
use sea_orm::{ColumnTrait, Condition, DbBackend, MockDatabase, sea_query::PostgresQueryBuilder};

#[test]
fn filters_and_authored_order_survive_exact_page_bounds() {
    let db = MockDatabase::new(DbBackend::Postgres).into_connection();
    let prepared = EntityStore::<post::Entity>::new(&db)
        .query()
        .filter(post::Column::Published.eq(true))
        .filter(
            Condition::any()
                .add(post::Column::Title.contains("rust"))
                .add(post::Column::Id.eq(7)),
        )
        .order_by_desc(post::Column::Title)
        .prepare_page(2, 20)
        .unwrap();
    let sql = prepared.statement().to_string(PostgresQueryBuilder);
    assert!(sql.contains(r#""published" = TRUE"#), "{sql}");
    assert!(sql.contains(" OR "), "{sql}");
    assert!(
        sql.ends_with(r#"ORDER BY "storage_posts"."title" DESC LIMIT 20 OFFSET 20"#),
        "{sql}"
    );
    assert!(!sql.contains("COUNT"));
    assert!(db.into_transaction_log().is_empty());
}

#[tokio::test]
async fn pagination_performs_one_read_without_trimming_or_counting() {
    let rows: Vec<_> = (20..40)
        .map(|id| post::Model {
            id,
            title: "x".into(),
            published: true,
        })
        .collect();
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([rows.clone(), vec![]])
        .into_connection();
    let query = EntityStore::<post::Entity>::new(&db)
        .query()
        .order_by_asc(post::Column::Id);
    let page = query.paginate(2, 20).await.unwrap();
    assert_eq!(page.items, rows);
    assert_eq!((page.page_number, page.page_size), (2, 20));
    let empty = EntityStore::<post::Entity>::new(&db)
        .query()
        .order_by_desc(post::Column::Id)
        .paginate(99, 20)
        .await
        .unwrap();
    assert!(empty.items.is_empty());
    assert_eq!(empty.page_number, 99);
    let logs = db.into_transaction_log();
    assert_eq!(logs.len(), 2);
    assert!(!format!("{logs:?}").contains("COUNT"));
}

#[tokio::test]
async fn invalid_windows_and_missing_order_execute_no_sql() {
    let db = MockDatabase::new(DbBackend::Postgres).into_connection();
    for (number, size, kind) in [
        (1, 20, StorageErrorKind::MissingOrder),
        (0, 20, StorageErrorKind::InvalidPagination),
        (1, 0, StorageErrorKind::InvalidPagination),
        (3, u64::MAX, StorageErrorKind::InvalidPagination),
    ] {
        let error = EntityStore::<post::Entity>::new(&db)
            .query()
            .paginate(number, size)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), kind);
    }
    assert!(db.into_transaction_log().is_empty());
}

#[test]
fn composite_order_is_not_extended_or_rewritten() {
    let db = MockDatabase::new(DbBackend::Postgres).into_connection();
    let prepared = EntityStore::<composite::Entity>::new(&db)
        .query()
        .order_by_desc(composite::Column::Id)
        .order_by_asc(composite::Column::TenantId)
        .prepare_page(1, 5)
        .unwrap();
    let sql = prepared.statement().to_string(PostgresQueryBuilder);
    assert!(sql.ends_with(r#"ORDER BY "storage_composite"."id" DESC, "storage_composite"."tenant_id" ASC LIMIT 5 OFFSET 0"#),"{sql}");
}

#[test]
fn representable_sizes_have_no_framework_cap_or_lookahead_overflow() {
    let db = MockDatabase::new(DbBackend::Postgres).into_connection();
    for size in [1_000_000, u64::MAX] {
        let prepared = EntityStore::<post::Entity>::new(&db)
            .query()
            .order_by_asc(post::Column::Id)
            .prepare_page(1, size)
            .unwrap();
        let sql = prepared.statement().to_string(PostgresQueryBuilder);
        assert!(sql.ends_with(&format!("LIMIT {size} OFFSET 0")), "{sql}");
    }
}

#[tokio::test]
async fn one_allows_unspecified_order_and_does_not_inject_a_default() {
    let row = post::Model {
        id: 3,
        title: "x".into(),
        published: true,
    };
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row.clone()]])
        .into_connection();
    assert_eq!(
        EntityStore::<post::Entity>::new(&db)
            .query()
            .one()
            .await
            .unwrap(),
        Some(row)
    );
    let (connection, statement) = EntityStore::<post::Entity>::new(&db).query().prepare_one();
    assert!(std::ptr::eq(connection, &db));
    let sql = statement.to_string(PostgresQueryBuilder);
    assert!(sql.ends_with("LIMIT 1"), "{sql}");
    assert!(!sql.contains("ORDER BY"));
}
