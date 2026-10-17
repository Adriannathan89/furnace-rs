//! Configuration and named-event wire contracts.
use furnace_rs_core::{Config, ConfigBuilder, MapSource};
use furnace_rs_socket::{SocketEnvelope, SocketServerConfig};
fn config(values: &[(&str, &str)]) -> Config {
    ConfigBuilder::new()
        .source(MapSource::new("test", values.iter().copied()))
        .build()
        .unwrap()
}
#[test]
fn requires_explicit_address() {
    assert!(SocketServerConfig::parse(&Config::empty()).is_err());
}
#[test]
fn defaults_match_approved_limits() {
    let c = SocketServerConfig::parse(&config(&[("socket.address", "127.0.0.1:0")])).unwrap();
    assert_eq!(c.max_message_bytes, 1_048_576);
    assert_eq!(c.queue_messages, 256);
    assert_eq!(c.queue_bytes, 4_194_304);
    assert_eq!(c.heartbeat_interval.as_secs(), 30);
    assert_eq!(c.pong_timeout.as_secs(), 10);
    assert_eq!(c.shutdown_timeout.as_secs(), 10);
}
#[test]
fn invalid_limits_rejected() {
    for (key, value) in [
        ("socket.queue_messages", "0"),
        ("socket.queue_bytes", "-1"),
        ("socket.max_message_bytes", "garbage"),
        ("socket.lease_refresh_ms", "15000"),
    ] {
        assert!(
            SocketServerConfig::parse(&config(&[("socket.address", "127.0.0.1:0"), (key, value)]))
                .is_err(),
            "{key}"
        );
    }
}
#[test]
fn protocol_roundtrip_and_invalid_messages() {
    let e = SocketEnvelope::parse(r#"{"event":"chat.send","id":"r1","data":{"text":"hello"}}"#)
        .unwrap();
    assert_eq!(e.event, "chat.send");
    assert_eq!(e.id.as_deref(), Some("r1"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&e.to_text().unwrap()).unwrap()["data"]["text"],
        "hello"
    );
    for bad in [
        "null",
        "{}",
        r#"{"event":"","data":{}}"#,
        r#"{"event":"x","event":"y","data":{}}"#,
    ] {
        assert!(SocketEnvelope::parse(bad).is_err(), "{bad}");
    }
}
#[test]
fn configuration_debug_redacts_secrets() {
    let c = config(&[
        ("socket.address", "127.0.0.1:0"),
        (
            "socket.redis.url",
            "redis://user:private-password-sentinel@localhost",
        ),
        ("socket.namespace", "test"),
    ]);
    if let Ok(c) = SocketServerConfig::parse(&c) {
        assert!(!format!("{c:?}").contains("private-password-sentinel"));
    }
}
#[test]
fn rejects_queue_capacities_that_tokio_cannot_represent() {
    let huge = usize::MAX.to_string();
    let oversized = (u64::from(u32::MAX) + 1).to_string();
    for (key, value) in [
        ("socket.queue_messages", huge.as_str()),
        ("socket.queue_bytes", oversized.as_str()),
    ] {
        assert!(
            SocketServerConfig::parse(&config(&[("socket.address", "127.0.0.1:0"), (key, value)]))
                .is_err()
        );
    }
}
#[cfg(feature = "redis")]
#[test]
fn redis_tls_cannot_disable_certificate_verification() {
    assert!(
        SocketServerConfig::parse(&config(&[
            ("socket.address", "127.0.0.1:0"),
            ("socket.namespace", "test"),
            ("socket.redis.url", "rediss://localhost:6379/#insecure")
        ]))
        .is_err()
    );
}
