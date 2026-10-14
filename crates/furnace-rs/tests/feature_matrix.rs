//! Cargo feature-composition regression tests.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn dependency_tree(features: &str) -> String {
    let root = workspace_root();
    let output = Command::new(env!("CARGO"))
        .current_dir(root)
        .args([
            "tree",
            "-e",
            "normal",
            "-p",
            "furnace-rs",
            "--no-default-features",
            "--features",
            features,
        ])
        .output()
        .expect("cargo tree should start");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout).expect("cargo output should be UTF-8")
}

fn default_dependency_tree() -> String {
    let output = Command::new(env!("CARGO"))
        .current_dir(workspace_root())
        .args(["tree", "-e", "normal", "-p", "furnace-rs"])
        .output()
        .expect("cargo tree should start");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn default_and_core_only_facades_exclude_diesel() {
    for tree in [default_dependency_tree(), dependency_tree("")] {
        for forbidden in [
            "diesel v",
            "deadpool-diesel v",
            "diesel_migrations v",
            "pq-sys v",
        ] {
            assert!(!tree.contains(forbidden), "{forbidden} in {tree}");
        }
    }
    let manifest = std::fs::read_to_string(workspace_root().join("crates/furnace-rs/Cargo.toml"))
        .expect("facade manifest should exist");
    assert!(!manifest.contains("diesel.workspace"));
}

#[test]
fn jwt_only_excludes_http_and_database_dependencies() {
    let tree = dependency_tree("jwt");
    for forbidden in [
        "axum v",
        "axum-extra v",
        "tower-http v",
        "diesel v",
        "deadpool-diesel v",
        "diesel_migrations v",
    ] {
        assert!(
            !tree.contains(forbidden),
            "unexpected dependency: {forbidden}\n{tree}"
        );
    }
}

#[test]
fn grpc_jwt_does_not_enable_http_adapter_or_cookie_dependencies() {
    let tree = dependency_tree("grpc,jwt");
    assert!(tree.contains("tonic v"));
    assert!(tree.contains("jsonwebtoken v"));
    for forbidden in [
        "axum-extra v",
        "tower-http v",
        "cookie v",
        "sea-orm v",
        "protoc-bin-vendored v",
    ] {
        assert!(
            !tree.contains(forbidden),
            "unexpected dependency: {forbidden}\n{tree}"
        );
    }
}

#[test]
fn grpc_stays_opt_in_for_default_and_jwt_only_consumers() {
    for tree in [default_dependency_tree(), dependency_tree("jwt")] {
        assert!(
            !tree.contains("tonic v"),
            "unexpected gRPC dependency\n{tree}"
        );
    }
}

#[test]
fn http_includes_validation_and_rest_without_database_or_jwt() {
    let tree = dependency_tree("http");
    for required in [
        "axum v",
        "axum-extra v",
        "furnace-rs-common-macros v",
        "tower-http v",
    ] {
        assert!(
            tree.contains(required),
            "missing HTTP validation or REST dependency: {required}\n{tree}"
        );
    }
    for forbidden in [
        "diesel v",
        "deadpool-diesel v",
        "diesel_migrations v",
        "jsonwebtoken v",
        "cookie v",
    ] {
        assert!(
            !tree.contains(forbidden),
            "unexpected HTTP dependency: {forbidden}\n{tree}"
        );
    }
}

#[test]
fn cookies_include_http_but_not_jwt_or_database() {
    let tree = dependency_tree("cookies");
    assert!(tree.contains("axum v"));
    assert!(tree.contains("cookie v"));
    assert!(!tree.contains("jsonwebtoken v"));
    assert!(!tree.contains("diesel v"));
}

#[test]
fn generated_minimal_project_selects_only_http_and_tokio() {
    let template = std::fs::read_to_string(
        workspace_root().join("crates/furnace-rs-cli/src/scaffold/templates/Cargo.toml.txt"),
    )
    .expect("generated project manifest template should exist");

    assert!(
        template.contains(
            "furnace = { package = \"furnace-rs\", version = \"={{furnace_version}}\", default-features = false, features = [\"http\", \"runtime-tokio\"] }"
        ),
        "the generated project must select exactly HTTP and Tokio support"
    );
    for forbidden in [
        "database",
        "diesel",
        "deadpool",
        "jwt",
        "jsonwebtoken",
        "cookie",
        "migration",
    ] {
        assert!(
            !template.contains(forbidden),
            "the generated minimal project must not select {forbidden}"
        );
    }
}

#[test]
fn grpc_implementation_is_owned_by_dedicated_packages() {
    let manifest =
        std::fs::read_to_string(workspace_root().join("crates/furnace-rs-common/Cargo.toml"))
            .unwrap();
    for forbidden in [
        "grpc =",
        "grpc-tls =",
        "tonic =",
        "prost =",
        "tonic-prost.workspace",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "common still owns {forbidden}"
        );
    }
    let tree = dependency_tree("grpc,jwt,grpc-tls");
    assert!(tree.contains("furnace-rs-grpc v"));
    assert!(tree.contains("furnace-rs-grpc-macros v"));
    for tree in [default_dependency_tree(), dependency_tree("jwt")] {
        assert!(!tree.contains("furnace-rs-grpc v"));
        assert!(!tree.contains("tonic v"));
    }
}

#[test]
fn grpc_feature_and_consumer_matrix_keeps_build_tools_out_of_runtime() {
    for features in [
        "",
        "http",
        "grpc",
        "grpc,jwt",
        "grpc,grpc-tls",
        "http,grpc,jwt,grpc-tls",
    ] {
        let tree = dependency_tree(features);
        assert_eq!(
            tree.contains("furnace-rs-grpc v"),
            features.contains("grpc")
        );
        assert_eq!(tree.contains("jsonwebtoken v"), features.contains("jwt"));
        assert_eq!(
            tree.contains("tokio-rustls v"),
            features.contains("grpc-tls")
        );
        for forbidden in [
            "tonic-prost-build v",
            "prost-build v",
            "protoc-bin-vendored v",
        ] {
            assert!(!tree.contains(forbidden), "{features}: {forbidden}\n{tree}");
        }
    }
}

#[test]
fn core_retains_toml_without_transport_or_direct_serde_dependencies() {
    let output = Command::new(env!("CARGO"))
        .current_dir(workspace_root())
        .args([
            "tree",
            "--locked",
            "-e",
            "normal",
            "-p",
            "furnace-rs-core",
            "--no-default-features",
        ])
        .output()
        .expect("cargo tree should start");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).unwrap();
    assert!(tree.contains("toml v"));
    for forbidden in [
        "tonic v",
        "prost v",
        "axum v",
        "http v",
        "jsonwebtoken v",
        "cookie v",
    ] {
        assert!(!tree.contains(forbidden), "{forbidden} in {tree}");
    }
    let manifest =
        std::fs::read_to_string(workspace_root().join("crates/furnace-rs-core/Cargo.toml"))
            .unwrap();
    assert!(!manifest.contains("serde.workspace"));
    assert!(!manifest.contains("serde ="));
}
