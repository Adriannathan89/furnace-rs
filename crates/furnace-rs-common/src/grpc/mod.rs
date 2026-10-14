//! Optional protobuf-first gRPC server and client integration.
mod config;
mod descriptor;
mod error;
pub(crate) mod preflight;
mod router;
mod scope;
mod service;
pub use router::build_grpc_routes;
#[doc(hidden)]
pub use router::{GrpcRegistrationContext, GrpcServiceRegistrar};

#[doc(hidden)]
pub use descriptor::{
    GrpcGuard, GrpcMethodOverride, GrpcServiceDescriptor, GrpcServiceRegistration,
};
pub use scope::{GrpcApplicationScope, GrpcMethodRecord, ScopedGrpcService};
pub use service::{GrpcCauldronExt, GrpcService};

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
