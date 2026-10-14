//! Shared runtime error compatibility.
#![cfg(feature = "transport-runtime")]
use furnace_rs_common::runtime::ApplicationRuntimeError;
use std::error::Error;

#[test]
fn neutral_transport_error_retains_the_original_cause() {
    let error = ApplicationRuntimeError::TransportServe {
        transport: "grpc",
        source: Box::new(std::io::Error::other("original cause")),
    };
    assert_eq!(error.source().unwrap().to_string(), "original cause");
    assert!(error.source().unwrap().is::<std::io::Error>());
}
