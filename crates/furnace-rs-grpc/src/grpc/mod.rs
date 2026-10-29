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
pub use service::GrpcService;

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

pub(crate) use client::parse_timeout;
#[doc(hidden)]
pub use client::{ErasedTokenProvider, GrpcClientLifecycle};
pub use client::{GrpcChannel, GrpcTokenProvider, set_grpc_bearer_token};
#[doc(hidden)]
pub use descriptor::{GrpcClientDescriptor, GrpcClientRegistration};

mod tls;

pub use tls::{client_endpoint, configure_grpc_server};

mod health;
pub(crate) mod reflection;

#[doc(hidden)]
pub use descriptor::{GrpcReflectionDescriptor, GrpcReflectionRegistration};
pub use health::{GrpcHealth, install_grpc_infrastructure};
pub use reflection::reflection_descriptors;
