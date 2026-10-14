//! Protobuf-first managed gRPC services, named clients, and native Tonic integration.
#![forbid(unsafe_code)]
/// Shared neutral Passport/JWT and metadata integration contracts.
#[doc(hidden)]
pub use furnace_rs_common as common;
/// Shared dependency injection and application lifecycle contracts.
pub use furnace_rs_core as core;
/// Generated service, client, and root reflection registration attributes.
pub use furnace_rs_grpc_macros::{grpc_client, grpc_reflection, grpc_service};
/// Native transport implementation and protobuf registration contracts.
pub mod grpc;
pub use grpc::*;
#[cfg(test)]
#[path = "grpc/config_tests.rs"]
mod grpc_config_tests;
/// Conventional transport selection configuration.
pub mod runtime;
/// Shared Passport and JWT contracts for optional gRPC admission protection.
#[cfg(feature = "jwt")]
pub use furnace_rs_common::{
    Authenticated, ClaimsPrincipal, ErasedAuthentication, FURNACE121, FURNACE130, FURNACE131,
    GuardCatalog, GuardDescriptor, GuardPolicy, JwtClaims, JwtService, JwtSignOptions,
    JwtTokenKind, JwtValidation, PassportContext, PassportError, PassportErrorKind,
    PassportPrincipal, PassportResult, PassportStrategy, PassportStrategyAdapter,
    PassportStrategyBinding, PassportStrategyCatalog, PassportStrategyFuture, TokenSource,
    VerifiedToken, guard, passport_strategy,
};
/// Automatic startup and explicit shared runtime composition.
pub use runtime::{
    ApplicationRuntimeError, FurnaceBurnExt, RuntimeBindings, RuntimeGrpcExt, RuntimeRoutes,
    RuntimeSelection, serve_transports,
};
