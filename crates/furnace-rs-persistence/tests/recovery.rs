//! Recovery checks against an isolated PostgreSQL instance.
#![cfg(feature = "sea-orm-postgres")]

use furnace_rs_persistence::{
    DatabaseFactory,
    sea_orm::{DatabaseConnection, SeaOrmPostgres},
};
use sea_orm::{ConnAcquireErr, ConnectionTrait, DbBackend, DbErr, Statement, TransactionTrait};
use std::{sync::Arc, time::Duration};

async fn database() -> DatabaseConnection {
    let url = std::env::var("FURNACE_TEST_DATABASE_URL").expect("isolated PostgreSQL URL required");
    let mut connector = SeaOrmPostgres::new(url);
    connector
        .options_mut()
        .max_connections(1)
        .min_connections(1)
        .acquire_timeout(Duration::from_millis(300))
        .sqlx_logging(false);
    DatabaseFactory.provide(connector).await.unwrap()
}

async fn backend(database: &DatabaseConnection) -> i32 {
    database
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_owned(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap()
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL through FURNACE_TEST_DATABASE_URL"]
async fn terminated_idle_connection_is_replaced_without_rebuilding_the_application() {
    let db = database().await;
    let original = backend(&db).await;
    let admin = database().await;
    let row = admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_terminate_backend($1) AS terminated",
            [original.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(row.try_get::<bool>("", "terminated").unwrap());
    assert_ne!(backend(&db).await, original);
    db.ping().await.unwrap();
    db.close_by_ref().await.unwrap();
    admin.close_by_ref().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL through FURNACE_TEST_DATABASE_URL"]
async fn pool_acquisition_timeout_does_not_permanently_consume_capacity() {
    let db = database().await;
    let transaction = db.begin().await.unwrap();
    assert!(matches!(
        db.ping().await,
        Err(DbErr::ConnectionAcquire(ConnAcquireErr::Timeout))
    ));
    transaction.rollback().await.unwrap();
    db.ping().await.unwrap();
    db.close_by_ref().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL through FURNACE_TEST_DATABASE_URL"]
async fn query_error_and_cancelled_query_leave_the_pool_usable() {
    let db = Arc::new(database().await);
    assert!(db.execute_unprepared("SELECT 1 / 0").await.is_err());
    db.ping().await.unwrap();
    let pid = backend(&db).await;
    let admin = database().await;
    let query_database = Arc::clone(&db);
    let mut query = tokio::spawn(async move {
        query_database
            .execute_unprepared("SELECT pg_sleep(1)")
            .await
    });
    // Confirm cancellation targets an executing query, rather than pool acquisition.
    let observed = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if admin.query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT pid FROM pg_stat_activity WHERE pid = $1 AND state = 'active' AND query = 'SELECT pg_sleep(1)'",
                [pid.into()],
            )).await.unwrap().is_some() { break; }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }).await;
    if observed.is_err() {
        query.abort();
        let _ = query.await;
        panic!("PostgreSQL did not observe the running query before cancellation");
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut query)
            .await
            .is_err()
    );
    query.abort();
    assert!(query.await.unwrap_err().is_cancelled());
    // Protocol cleanup may complete the cancelled server query before reuse.
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if db.ping().await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("cancelled query must release capacity and restore a usable connection");
    db.close_by_ref().await.unwrap();
    admin.close_by_ref().await.unwrap();
}
