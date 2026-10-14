//! Optional protobuf-first gRPC server and client integration.
mod config;
mod error;

pub use config::{
    GrpcClientConfig, GrpcClientTlsConfig, GrpcConnectionMode, GrpcServerConfig,
    GrpcServerTlsConfig,
};
pub use error::{
    FURNACE150, FURNACE151, FURNACE152, FURNACE153, GrpcClientError, GrpcClientErrorKind,
    GrpcClientResult,
};
/// Native Tonic contracts for requests, responses, statuses, and streams.
pub use tonic;
