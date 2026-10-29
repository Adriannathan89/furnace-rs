//! Pure transport configuration regression contracts.
use crate::grpc::{GrpcClientConfig, GrpcConnectionMode, GrpcServerConfig};
use crate::runtime::RuntimeSelection;
use furnace_rs_core::{Config, ConfigBuilder, MapSource, TomlSource};
use std::time::Duration;

fn config(values: &[(&str, &str)]) -> Config {
    ConfigBuilder::new()
        .source(MapSource::new("test", values.iter().copied()))
        .build()
        .unwrap()
}

#[test]
fn grpc_config_defaults() {
    let server = GrpcServerConfig::parse(&Config::empty()).unwrap();
    assert_eq!(server.address().to_string(), "127.0.0.1:50051");
    assert_eq!(server.max_encoding_message_bytes(), 4_194_304);
    assert_eq!(server.max_decoding_message_bytes(), 4_194_304);
    assert_eq!(server.shutdown_timeout(), Duration::from_secs(30));
    assert_eq!(server.tls_handshake_timeout(), Duration::from_secs(10));
    let client = GrpcClientConfig::parse(
        &config(&[("grpc.clients.users.endpoint", "http://127.0.0.1:50051")]),
        "users",
    )
    .unwrap();
    assert_eq!(client.connection_mode(), GrpcConnectionMode::Lazy);
    assert_eq!(client.connect_timeout(), Duration::from_secs(10));
    assert_eq!(client.tls_handshake_timeout(), Duration::from_secs(10));
    assert_eq!(client.request_timeout(), None);
    assert_eq!(client.max_decoding_message_bytes(), 4_194_304);
}

#[test]
fn transport_selection_rejects_empty_duplicate_unknown_or_uncompiled() {
    assert_eq!(
        RuntimeSelection::parse(&Config::empty(), true, true).unwrap(),
        RuntimeSelection {
            http: true,
            grpc: false,
            socket: false
        }
    );

    for values in [vec![], vec!["grpc", "grpc"], vec!["ws"]] {
        let c = ConfigBuilder::new()
            .source(
                MapSource::new("test", [] as [(&str, &str); 0])
                    .with_string_array("furnace.transports", values),
            )
            .build()
            .unwrap();
        assert!(RuntimeSelection::parse(&c, true, true).is_err());
    }

    let c = ConfigBuilder::new()
        .source(
            MapSource::new("test", [] as [(&str, &str); 0])
                .with_string_array("furnace.transports", ["grpc"]),
        )
        .build()
        .unwrap();
    assert!(RuntimeSelection::parse(&c, true, false).is_err());
    assert_eq!(
        RuntimeSelection::parse(&c, false, true).unwrap(),
        RuntimeSelection {
            http: false,
            grpc: true,
            socket: false
        }
    );
    assert!(
        RuntimeSelection::parse(&config(&[("furnace.transports", "grpc")]), true, true).is_err()
    );
}

#[test]
fn grpc_config_rejects_zero_overflow_partial_tls_and_custom_ca_only_without_cas() {
    for (key, value) in [
        ("grpc.server.port", "0"),
        ("grpc.server.port", "65536"),
        ("grpc.server.host", "localhost"),
        ("grpc.server.max_decoding_message_bytes", "0"),
        ("grpc.server.shutdown_timeout_ms", "0"),
        ("grpc.server.shutdown_timeout_ms", "18446744073709551616"),
        ("grpc.server.tls.certificate", "private-token-sentinel"),
        ("grpc.server.tls", "invalid"),
        ("grpc.server.host", "private-token-sentinel"),
    ] {
        let error = GrpcServerConfig::parse(&config(&[(key, value)])).expect_err(key);
        assert!(!error.to_string().contains("private-token-sentinel"));
    }

    for (key, value) in [
        ("grpc.clients.users.connect", "other"),
        ("grpc.clients.users.connect_timeout_ms", "0"),
        ("grpc.clients.users.tls.custom_ca_only", "true"),
        (
            "grpc.clients.users.endpoint",
            "ftp://private-token-sentinel",
        ),
        (
            "grpc.clients.users.endpoint",
            "http://user:private-token-sentinel@localhost:50051",
        ),
    ] {
        let c = config(&[
            ("grpc.clients.users.endpoint", "http://127.0.0.1:50051"),
            (key, value),
        ]);
        assert!(GrpcClientConfig::parse(&c, "users").is_err());
    }

    assert!(GrpcClientConfig::parse(&Config::empty(), "users").is_err());
}

#[test]
fn grpc_configuration_paths_follow_source() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("furnace.toml");
    std::fs::write(
        &path,
        "[grpc.server.tls]\ncertificate = 'certs/server.pem'\nprivate_key = 'certs/server.key'\n",
    )
    .unwrap();
    let c = ConfigBuilder::new()
        .source(TomlSource::file(path))
        .build()
        .unwrap();

    #[cfg(feature = "tls")]
    {
        let s = GrpcServerConfig::parse(&c).unwrap();
        assert_eq!(
            s.tls().unwrap().certificate(),
            root.path().join("certs/server.pem")
        );
        assert_eq!(
            s.tls().unwrap().private_key(),
            root.path().join("certs/server.key")
        );
    }
    #[cfg(not(feature = "tls"))]
    assert!(GrpcServerConfig::parse(&c).is_err());
}

#[test]
fn grpc_config_rejects_scalar_tables() {
    for key in ["grpc", "grpc.server", "grpc.reflection"] {
        assert!(
            GrpcServerConfig::parse(&config(&[(key, "invalid")])).is_err(),
            "accepted scalar table {key}"
        );
    }
}

#[cfg(feature = "tls")]
#[test]
fn invalid_tls_server_name_is_rejected_without_startup_io() {
    let settings = config(&[
        ("grpc.clients.remote.endpoint", "https://localhost:50051"),
        (
            "grpc.clients.remote.tls.domain_name",
            "not a valid server name",
        ),
    ]);
    assert!(GrpcClientConfig::parse(&settings, "remote").is_err());
}
