//! Public PostgreSQL connector contract.
#![cfg(feature = "sea-orm-postgres")]

use furnace_rs_persistence::{
    DatabaseFactory, PersistenceErrorKind,
    sea_orm::{ConnectOptions, SeaOrmPostgres},
};

#[tokio::test]
async fn unsupported_schemes_fail_before_connecting() {
    for url in [
        "mysql://localhost/db",
        "sqlite://db.sqlite",
        "localhost/db",
        "",
    ] {
        let result = DatabaseFactory.provide(SeaOrmPostgres::new(url)).await;
        assert_eq!(
            result.unwrap_err().kind(),
            PersistenceErrorKind::UnsupportedScheme
        );
    }
}

#[tokio::test]
async fn native_zero_pool_capacity_returns_a_typed_error_without_panicking() {
    let mut options = ConnectOptions::new("postgres://user:pool-secret-password@localhost/db");
    options.max_connections(0).connect_lazy(true);
    let result = tokio::spawn(async move {
        DatabaseFactory
            .provide(SeaOrmPostgres::from_options(options))
            .await
    })
    .await
    .expect("zero pool capacity must return a typed error, not panic");
    let error = result.expect_err("zero pool capacity must fail before creating a pool");
    assert_eq!(error.kind(), PersistenceErrorKind::InvalidConfiguration);
    assert!(!format!("{error:?} {error}").contains("pool-secret-password"));

    let mut valid = SeaOrmPostgres::new("postgres://localhost/db");
    valid.options_mut().max_connections(1).connect_lazy(true);
    let connection = DatabaseFactory.provide(valid).await.unwrap();
    connection.close_by_ref().await.unwrap();
}

#[test]
fn connector_debug_never_discloses_url() {
    let url = "postgres://furnace_rs-secret-user:furnace_rs-secret-password@localhost/furnace_rs?token=furnace_rs-secret-query";
    let mut connector = SeaOrmPostgres::new(url);
    assert_eq!(connector.options_mut().get_url(), url);
    assert!(!format!("{connector:?}").contains("furnace_rs-secret"));
    let from_options = SeaOrmPostgres::from_options(ConnectOptions::new(url));
    assert!(!format!("{from_options:?}").contains("furnace_rs-secret"));
}

#[tokio::test]
async fn unrepresentable_pool_deadline_is_rejected_without_panicking() {
    let mut options =
        ConnectOptions::new("postgres://test-user:timeout-secret-password@127.0.0.1:1/db");
    options.acquire_timeout(std::time::Duration::MAX);
    let result = tokio::spawn(async move {
        DatabaseFactory
            .provide(SeaOrmPostgres::from_options(options))
            .await
    })
    .await
    .expect("invalid duration must return a typed error, not panic");
    let error = result.expect_err("unrepresentable deadline must fail before connecting");
    assert_eq!(error.kind(), PersistenceErrorKind::InvalidConfiguration);
    assert!(!format!("{error:?} {error}").contains("timeout-secret-password"));
}

#[tokio::test]
async fn native_options_reject_all_overflowing_deadlines_even_with_lazy_connections() {
    for field in ["connect", "acquire", "idle", "lifetime"] {
        let mut connector = SeaOrmPostgres::new("postgres://localhost/db");
        let options = connector.options_mut();
        options.connect_lazy(true);
        match field {
            "connect" => {
                options.connect_timeout(std::time::Duration::MAX);
            }
            "acquire" => {
                options.acquire_timeout(std::time::Duration::MAX);
            }
            "idle" => {
                options.idle_timeout(std::time::Duration::MAX);
            }
            "lifetime" => {
                options.max_lifetime(std::time::Duration::MAX);
            }
            _ => unreachable!(),
        }
        assert_eq!(
            DatabaseFactory.provide(connector).await.unwrap_err().kind(),
            PersistenceErrorKind::InvalidConfiguration
        );
    }
}

#[tokio::test]
async fn valid_native_timeouts_and_disabled_reaping_preserve_lazy_connections() {
    let mut connector = SeaOrmPostgres::new("postgres://localhost/db");
    connector
        .options_mut()
        .connect_lazy(true)
        .connect_timeout(std::time::Duration::from_secs(2))
        .acquire_timeout(std::time::Duration::from_secs(2))
        .idle_timeout(None)
        .max_lifetime(None);
    let connection = DatabaseFactory.provide(connector).await.unwrap();
    connection.close_by_ref().await.unwrap();
}

#[tokio::test]
async fn zero_maintenance_deadlines_are_rejected_before_creating_a_pool() {
    for field in ["idle", "lifetime"] {
        let mut connector = SeaOrmPostgres::new("postgres://localhost/db");
        let options = connector.options_mut();
        options.connect_lazy(true);
        match field {
            "idle" => {
                options.idle_timeout(std::time::Duration::ZERO);
            }
            "lifetime" => {
                options.max_lifetime(std::time::Duration::ZERO);
            }
            _ => unreachable!(),
        }
        let result = DatabaseFactory.provide(connector).await;
        // Close an unexpectedly accepted pool immediately; never run a busy-loop workload.
        if let Ok(connection) = &result {
            connection.close_by_ref().await.unwrap();
        }
        assert_eq!(
            result.unwrap_err().kind(),
            PersistenceErrorKind::InvalidConfiguration
        );
    }
}
