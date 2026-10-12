//! Repeatable core and persistence security contracts, without a live database.

use std::{
    error::Error as _,
    time::{Duration, Instant},
};

use furnace_rs_core::{ConfigBuilder, Diagnostic, Error, FURNACE006, Injector, MapSource};
use furnace_rs_persistence::{
    DatabaseFactory, PersistenceError, PersistenceErrorKind,
    sea_orm::{ConnectOptions, SeaOrmPostgres},
};

const SECRET: &str = "postgres://fixture-user:infrastructure-secret-password@127.0.0.1:1/db";
#[path = "support/tracing_capture.rs"]
mod tracing_capture;
use tracing::instrument::WithSubscriber;
const TIMEOUT_KEYS: [&str; 4] = [
    "persistence.seaorm.connect_timeout_seconds",
    "persistence.seaorm.acquire_timeout_seconds",
    "persistence.seaorm.idle_timeout_seconds",
    "persistence.seaorm.max_lifetime_seconds",
];

#[tokio::test]
async fn core_and_database_security_contracts() {
    let rounds: usize = std::env::var("FURNACE_SECURITY_ROUNDS")
        .unwrap_or_else(|_| "2".into())
        .parse()
        .expect("rounds must be an integer");
    assert!(
        (1..=10_000).contains(&rounds),
        "rounds must be between 1 and 10000"
    );
    let started = Instant::now();
    let mut core_failures = 0;
    let mut config_failures = 0;
    let mut native_failures = 0;
    let mut control_failures = 0;
    let mut trace_failures = 0;
    for _ in 0..rounds {
        let cause = Error::with_source(
            Diagnostic::new(FURNACE006, "connection failed", "database unavailable"),
            std::io::Error::other(SECRET),
        );
        let error = Error::with_source(
            Diagnostic::new(FURNACE006, "provider failed", "construction aborted"),
            cause,
        );
        let retained = error
            .source()
            .and_then(|error| error.downcast_ref::<Error>())
            .and_then(|error| error.source())
            .is_some_and(|source| source.to_string() == SECRET);
        if !retained
            || [
                format!("{error}"),
                format!("{error:?}"),
                format!("{error:#?}"),
            ]
            .iter()
            .any(|rendered| rendered.contains("infrastructure-secret-password"))
        {
            core_failures += 1;
        }
        for key in TIMEOUT_KEYS {
            let config = ConfigBuilder::new()
                .source(MapSource::new(
                    "security-fixture",
                    [
                        ("persistence.seaorm.url", SECRET),
                        (key, "18446744073709551615"),
                    ],
                ))
                .build()
                .unwrap();
            let valid_rejection = match SeaOrmPostgres::inject((config,)).await {
                Err(error) => {
                    let safe =
                        !format!("{error:?} {error}").contains("infrastructure-secret-password");
                    safe && error
                        .source()
                        .and_then(|source| source.downcast_ref::<PersistenceError>())
                        .is_some_and(|source| {
                            source.kind() == PersistenceErrorKind::InvalidConfiguration
                        })
                }
                Ok(_) => false,
            };
            if !valid_rejection {
                config_failures += 1;
            }
            let config = ConfigBuilder::new()
                .source(MapSource::new(
                    "security-fixture",
                    [("persistence.seaorm.url", SECRET), (key, "2")],
                ))
                .build()
                .unwrap();
            if SeaOrmPostgres::inject((config,)).await.is_err() {
                control_failures += 1;
            }
        }
        let mut options = ConnectOptions::new(SECRET);
        options.acquire_timeout(Duration::MAX);
        let result = tokio::spawn(async move {
            DatabaseFactory
                .provide(SeaOrmPostgres::from_options(options))
                .await
        })
        .await;
        let valid_rejection = match result {
            Ok(Err(error)) => {
                error.kind() == PersistenceErrorKind::InvalidConfiguration
                    && !format!("{error:?} {error}").contains("infrastructure-secret-password")
            }
            _ => false,
        };
        if !valid_rejection {
            native_failures += 1;
        }
        let subscriber = tracing_capture::RecordingSubscriber::default();
        let result = async {
            tracing::trace!("before connection");
            let result = DatabaseFactory
                .provide(SeaOrmPostgres::new(
                    "postgres://user:infrastructure-secret-password@127.0.0.1:not-a-port/db",
                ))
                .await;
            tracing::trace!("after connection");
            result
        }
        .with_subscriber(subscriber.clone())
        .await;
        let trace = subscriber.snapshot();
        if result.is_ok()
            || trace.contains("infrastructure-secret-password")
            || !trace.contains("before connection")
            || !trace.contains("after connection")
        {
            trace_failures += 1;
        }
    }
    let passed =
        core_failures + config_failures + native_failures + control_failures + trace_failures == 0;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    println!(
        concat!(
            "FURNACE_SECURITY_RESULT {{\"rounds\":{},\"elapsed_ms\":{:.3},",
            "\"core_source_redaction\":{{\"checks\":{},\"failures\":{}}},",
            "\"database_config_timeouts\":{{\"checks\":{},\"failures\":{}}},",
            "\"database_native_timeout\":{{\"checks\":{},\"failures\":{}}},",
            "\"valid_config_controls\":{{\"checks\":{},\"failures\":{}}},",
            "\"database_connection_trace\":{{\"checks\":{},\"failures\":{}}},\"passed\":{}}}"
        ),
        rounds,
        elapsed_ms,
        rounds,
        core_failures,
        rounds * 4,
        config_failures,
        rounds,
        native_failures,
        rounds * 4,
        control_failures,
        rounds,
        trace_failures,
        passed
    );
    assert!(
        passed,
        "core/database security contracts failed; see benchmark counts"
    );
}
