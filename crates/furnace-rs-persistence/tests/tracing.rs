//! Credentials must stay out of native database connection trace spans.
#![cfg(feature = "sea-orm-postgres")]

#[path = "support/tracing_capture.rs"]
mod tracing_capture;

use furnace_rs_persistence::{DatabaseFactory, sea_orm::SeaOrmPostgres};
use tracing::instrument::WithSubscriber;
use tracing_capture::RecordingSubscriber;

#[tokio::test]
async fn connection_trace_spans_do_not_record_credential_bearing_options() {
    let subscriber = RecordingSubscriber::default();
    // URL parsing fails immediately, so the test requires no database or socket.
    let url = "postgres://user:trace-secret-password@127.0.0.1:not-a-port/db";
    let result = async {
        tracing::trace!("before connection");
        let result = DatabaseFactory.provide(SeaOrmPostgres::new(url)).await;
        tracing::trace!("after connection");
        result
    }
    .with_subscriber(subscriber.clone())
    .await;
    assert!(result.is_err());
    let trace = subscriber.snapshot();
    assert!(trace.contains("before connection") && trace.contains("after connection"));
    assert!(
        !trace.contains("trace-secret-password"),
        "native connection tracing leaked credentials"
    );
}
