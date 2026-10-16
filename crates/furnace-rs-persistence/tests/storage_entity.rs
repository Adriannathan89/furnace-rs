//! Basic storage operations preserve native entity and hook behavior.
#![cfg(feature = "sea-orm-storage")]
#[path = "support/storage_entities.rs"]
mod entities;

use entities::{composite, post};
use furnace_rs_persistence::storage::{EntityStore, StorageErrorKind};
use sea_orm::{
    ActiveValue::{Set, Unchanged},
    DbBackend, DbErr, IntoActiveModel, MockDatabase, MockExecResult,
};
use std::error::Error;

#[tokio::test]
async fn active_model_save_hooks_run_for_insert_and_update() {
    use entities::hooked;
    use std::sync::atomic::Ordering;
    let row = hooked::Model {
        id: 1,
        title: "before".into(),
    };
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row.clone()], vec![row.clone()]])
        .into_connection();
    let store = EntityStore::<hooked::Entity>::new(&db);
    store
        .insert(hooked::ActiveModel {
            title: Set("input".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    store
        .update(hooked::ActiveModel {
            id: Set(1),
            title: Set("input".into()),
        })
        .await
        .unwrap();
    assert_eq!(hooked::BEFORE.load(Ordering::SeqCst), 2);
    assert_eq!(hooked::AFTER.load(Ordering::SeqCst), 2);
    let log = format!("{:?}", db.into_transaction_log());
    assert!(log.contains("before"));
    assert!(!log.contains("input"));
}

fn post(id: i32) -> post::Model {
    post::Model {
        id,
        title: "original".into(),
        published: true,
    }
}

#[tokio::test]
async fn lookup_and_delete_preserve_absence_semantics() {
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![post(7)], vec![]])
        .append_exec_results([
            MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            },
            MockExecResult::default(),
        ])
        .into_connection();
    let store = EntityStore::<post::Entity>::new(&db);
    assert_eq!(store.find_by_id(7).await.unwrap(), Some(post(7)));
    assert!(store.find_by_id(99).await.unwrap().is_none());
    assert!(store.delete_by_id(7).await.unwrap());
    assert!(!store.delete_by_id(99).await.unwrap());
    assert_eq!(db.into_transaction_log().len(), 4);
}

#[tokio::test]
async fn insert_and_update_use_native_returning_without_preflight() {
    let updated = post::Model {
        title: "changed".into(),
        ..post(7)
    };
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![post(7)], vec![updated.clone()]])
        .into_connection();
    let store = EntityStore::<post::Entity>::new(&db);
    assert_eq!(
        store
            .insert(post::ActiveModel {
                title: Set("original".into()),
                published: Set(true),
                ..Default::default()
            })
            .await
            .unwrap()
            .id,
        7
    );
    assert_eq!(
        store
            .update(post::ActiveModel {
                id: Unchanged(7),
                title: Set("changed".into()),
                published: Unchanged(true)
            })
            .await
            .unwrap(),
        updated
    );
    let logs = db.into_transaction_log();
    assert_eq!(logs.len(), 2);
    let rendered = format!("{logs:?}");
    assert!(rendered.contains("INSERT INTO"));
    assert!(rendered.contains("UPDATE"));
    assert!(!rendered.contains("SELECT"));
}

#[tokio::test]
async fn missing_update_is_typed_and_retains_native_cause() {
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([Vec::<post::Model>::new()])
        .into_connection();
    let error = EntityStore::<post::Entity>::new(&db)
        .update(post::ActiveModel {
            id: Set(7),
            title: Set("changed".into()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(error.kind(), StorageErrorKind::NotFound);
    assert!(matches!(
        error.source().unwrap().downcast_ref::<DbErr>(),
        Some(DbErr::RecordNotUpdated)
    ));
    assert_eq!(db.into_transaction_log().len(), 1);
}

#[tokio::test]
async fn composite_keys_are_scoped_and_missing_parts_fail_without_sql() {
    let model = composite::Model {
        tenant_id: 4,
        id: 7,
        title: "x".into(),
    };
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![model.clone()]])
        .into_connection();
    let store = EntityStore::<composite::Entity>::new(&db);
    assert_eq!(store.find_by_id((4, 7)).await.unwrap(), Some(model));
    let error = store
        .update(composite::ActiveModel {
            tenant_id: Set(4),
            title: Set("x".into()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(error.kind(), StorageErrorKind::Database);
    assert!(matches!(
        error.source().unwrap().downcast_ref::<DbErr>(),
        Some(DbErr::PrimaryKeyNotSet { .. })
    ));
    let logs = db.into_transaction_log();
    assert_eq!(logs.len(), 1);
    let rendered = format!("{logs:?}");
    assert!(rendered.contains("tenant_id"));
    assert!(rendered.contains(" AND "));
}

#[tokio::test]
async fn unchanged_model_and_database_failures_preserve_native_behavior() {
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![post(7)]])
        .into_connection();
    assert_eq!(
        EntityStore::<post::Entity>::new(&db)
            .update(post(7).into_active_model())
            .await
            .unwrap(),
        post(7)
    );
    let logs = db.into_transaction_log();
    assert_eq!(logs.len(), 1);
    assert!(format!("{logs:?}").contains("SELECT"));
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_errors([DbErr::Custom("secret".into())])
        .into_connection();
    let error = EntityStore::<post::Entity>::new(&db)
        .find_by_id(7)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), StorageErrorKind::Database);
    assert!(!format!("{error:?} {error}").contains("secret"));
}
