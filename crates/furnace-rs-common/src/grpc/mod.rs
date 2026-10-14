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

#[cfg(feature = "jwt")]
mod passport;
#[cfg(feature = "jwt")]
pub use passport::{GrpcPassportService, grpc_authenticated, grpc_verified_token, passport_status};

mod client;
#[doc(hidden)]
pub use client::{ErasedTokenProvider, GrpcClientLifecycle};
pub use client::{GrpcChannel, GrpcTokenProvider, set_grpc_bearer_token};
#[doc(hidden)]
pub use descriptor::{GrpcClientDescriptor, GrpcClientRegistration};
