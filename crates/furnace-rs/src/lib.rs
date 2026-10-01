//! Public furnace-rs facade and feature composition boundary.
//!
//! The default facade composes the HTTP and logger integrations with the
//! Tokio runtime. A root [`Cauldron`] selects explicitly registered providers,
//! controllers, routes, guards, strategies, and official auto-configurations
//! that belong to one application. Direct imports and unrestricted `pub`
//! exports govern access across cauldron boundaries.
//!
//! The standard startup path loads optional `.env`, optional `furnace.toml`, and
//! final `FURNACE_*` overrides from the current working directory in that order.
//! It builds the root module, configures the complete router, starts lifecycle
//! hooks, and serves the configured listener. The automatic server defaults are
//! `127.0.0.1:3000`; `[server.cors]` is an opt-in strict outer router layer:
//!
//! ```no_run
//! use furnace_rs::prelude::*;
//!
//! mod user {
//!     use furnace_rs::prelude::*;
//!
//!     #[cauldron]
//!     pub struct UserHttpCauldron;
//!     impl Cauldron for UserHttpCauldron {
//!         fn register(self) -> CauldronRegistration<Self> { CauldronRegistration::new(self) }
//!     }
//! }
//! use user::UserHttpCauldron;
//!
//! #[cauldron]
//! struct AppCauldron;
//! impl Cauldron for AppCauldron {
//!     fn register(self) -> CauldronRegistration<Self> { self.import(UserHttpCauldron) }
//! }
//!
//! #[furnace_rs::main]
//! async fn main() -> Result<(), HttpRuntimeError> {
//!     Furnace::burn::<AppCauldron>().await
//! }
//! ```
//!
//! Use the low-level builder for explicit configuration,
//! lifecycle hooks, native routers, or listener addresses. It never loads
//! conventional sources. [`build_router`] returns an unconfigured generated
//! router so native routes can be merged before [`configure_router`] or
//! [`serve_router`] applies application-wide configuration. A builder without
//! [`core::FurnaceBuilder::root`] retains complete-catalog compatibility.
//!
//! ```no_run
//! use furnace_rs::prelude::*;
//!
//! # #[cauldron]
//! # struct AppCauldron;
//! # impl Cauldron for AppCauldron {
//! #     fn register(self) -> CauldronRegistration<Self> { CauldronRegistration::new(self) }
//! # }
//! # async fn low_level(
//! #     config: Config,
//! #     native_router: furnace_rs::axum::Router,
//! # )
//! # -> Result<(), Box<dyn std::error::Error>> {
//! let mut builder = Furnace::builder_with_config(config);
//! builder.root::<AppCauldron>()?;
//! // builder.lifecycle_hook(MyHook);
//! let application = builder.build().await?;
//! let router = build_router(&application)?.merge(native_router);
//! serve_router(application, router, "127.0.0.1:0").await?;
//! # Ok(())
//! # }
//! ```
//!
//! The explicit address overrides `[server]` binding and may use port zero.
//! `configure_router` is the alternative for direct in-process router use after
//! the merge; do not pass an already configured router to `serve_router`.
//! Database provisioning is available through an explicitly imported
//! `furnace-rs-persistence` module. The retained [`AutoConfigurationReport`] records
//! expose redacted decision evidence only.
//!
//! Register custom access and refresh strategies as managed providers. FURNACE
//! verifies JWT cryptography, registered claims, and token kind before either
//! strategy receives typed claims:
//!
//! ```
//! # #[cfg(feature = "jwt")]
//! # mod jwt_strategy_example {
//! use furnace_rs::prelude::*;
//!
//! #[derive(serde::Deserialize)]
//! struct UserClaims { user_id: u64 }
//! struct UserPrincipal(u64);
//! impl PassportPrincipal for UserPrincipal {
//!     fn has_role(&self, role: &str) -> bool { role == "user" }
//!     fn has_permission(&self, permission: &str) -> bool {
//!         permission == "profile:read"
//!     }
//! }
//!
//! #[burner]
//! struct AccessStrategy;
//! #[passport_strategy(name = "jwt")]
//! impl PassportStrategy for AccessStrategy {
//!     type Claims = UserClaims;
//!     type Principal = UserPrincipal;
//!     const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;
//!     async fn validate(
//!         &self,
//!         _context: &PassportContext<'_>,
//!         claims: &JwtClaims<Self::Claims>,
//!     ) -> PassportResult<Self::Principal> {
//!         Ok(UserPrincipal(claims.custom.user_id))
//!     }
//! }
//!
//! #[burner]
//! struct RefreshStrategy;
//! #[passport_strategy(name = "jwt-refresh")]
//! impl PassportStrategy for RefreshStrategy {
//!     type Claims = UserClaims;
//!     type Principal = UserPrincipal;
//!     const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Refresh;
//!     async fn validate(
//!         &self,
//!         _context: &PassportContext<'_>,
//!         claims: &JwtClaims<Self::Claims>,
//!     ) -> PassportResult<Self::Principal> {
//!         Ok(UserPrincipal(claims.custom.user_id))
//!     }
//! }
//! # fn main() {}
//! # }
//! ```
//!
//! Route guards inherit field by field. Method clauses replace only supplied
//! fields, cookie sources select exactly one named cookie, and `skip` removes
//! an inherited guard:
//!
//! ```
//! # #[cfg(feature = "jwt")]
//! # mod jwt_guard_example {
//! use furnace_rs::prelude::*;
//!
//! struct UserPrincipal;
//! impl PassportPrincipal for UserPrincipal {
//!     fn has_role(&self, role: &str) -> bool { role == "user" }
//!     fn has_permission(&self, permission: &str) -> bool {
//!         permission == "profile:read"
//!     }
//! }
//! fn owns_profile(_: &UserPrincipal) -> bool { true }
//!
//! #[routes(prefix = "/users")]
//! #[guard(
//!     strategy = "jwt",
//!     principal = UserPrincipal,
//!     source = bearer,
//!     roles(any = ["user", "admin"]),
//! )]
//! trait UserRoutes {
//!     #[get("/profile")]
//!     #[guard(
//!         permissions(all = ["profile:read"]),
//!         predicate = owns_profile,
//!     )]
//!     async fn profile(&self, principal: Authenticated<UserPrincipal>);
//!
//!     #[post("/refresh")]
//!     #[guard(strategy = "jwt-refresh", source = cookie("refresh_token"))]
//!     async fn refresh(&self);
//!
//!     #[post("/login")]
//!     #[guard(skip)]
//!     async fn login(&self);
//! }
//! # fn main() {}
//! # }
//! ```

#![deny(missing_docs)]
#![forbid(unsafe_code)]

/// Re-exports the framework-neutral furnace-rs core.
pub use furnace_rs_core as core;

/// Version of the FURNACE framework facade used by downstream tooling.
pub const FRAMEWORK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Re-exports the asynchronous furnace-rs entry-point attribute.
pub use furnace_rs_core::main;

/// Registers an async Cargo test with a function-local fixture builder.
pub use furnace_rs_core::test;

/// Re-exports explicit cauldron registration and dependency declarations.
pub use furnace_rs_core::{
    Cauldron, CauldronRegistration, Furnace, FurnaceBuilder, burner, cauldron, element, storage,
};

/// Static guard policy metadata used by generated controller registration.
#[cfg(all(feature = "http", feature = "jwt"))]
#[doc(hidden)]
pub use furnace_rs_common::{GuardPolicy, SealEntry};
/// Re-exports static controller protection declarations.
#[cfg(feature = "http")]
pub use furnace_rs_common::{SealDefinition, SealRegistration, Sealable};

/// Re-exports explicit typed configuration, structured failures, and secret values.
pub use furnace_rs_core::{
    Configuration, ConfigurationErrors, ConfigurationIssue, ConfigurationResult, Secret,
};

/// Re-exports auto-configuration inspection records.
pub use furnace_rs_core::{
    AutoConfigurationConfigEvidence, AutoConfigurationReasonCode, AutoConfigurationReport,
    AutoConfigurationRequirement, AutoConfigurationStatus,
};

/// Re-exports root-module contracts and retained module-graph inspection records.
pub use furnace_rs_core::{
    CauldronGraph, CauldronImportDescriptor, CauldronImportEdge, CauldronNode, ProviderOwnership,
};

/// Re-exports enabled standard integrations.
#[cfg(any(
    feature = "http",
    feature = "jwt",
    feature = "cookies",
    feature = "logger"
))]
pub use furnace_rs_common as common;

/// Re-exports Axum for native HTTP runtime integration.
#[cfg(feature = "http")]
pub use furnace_rs_common::axum;

/// Re-exports the standard logger façade, backend contract, and global module.
#[cfg(feature = "logger")]
pub use furnace_rs_common::{
    ConsoleLoggerService, LogLevel, Logger, LoggerCauldron, LoggerService,
};

/// Re-exports strict cookie integration and the established cookie time types.
#[cfg(feature = "cookies")]
pub use furnace_rs_common::cookie;

/// Re-exports typed JWT claims, service, options, errors, and diagnostics.
#[cfg(feature = "jwt")]
pub use furnace_rs_common::{
    FURNACE120, FURNACE121, JwtAlgorithm, JwtClaims, JwtError, JwtErrorKind, JwtHeader, JwtResult,
    JwtService, JwtSignOptions, JwtTokenKind, JwtValidation, PassportConfig, RegisteredJwtClaims,
    VerifiedJwt,
};

/// Re-exports strict cookie extraction, response composition, and diagnostics.
#[cfg(feature = "cookies")]
pub use furnace_rs_common::{
    Cookie, CookieError, CookieErrorKind, CookieJar, CookieRejection, CookieResult, Expiration,
    FURNACE110, SameSite,
};

/// Re-exports HTTP request extractors and their typed-header support.
#[cfg(feature = "http")]
pub use furnace_rs_common::{Header, Json, Path, Query, Request, headers};

/// Re-exports standard HTTP response types.
#[cfg(feature = "http")]
pub use furnace_rs_common::{
    BadRequest, Conflict, Created, Forbidden, HttpError, HttpResult, InternalError, NoContent,
    NotFound, Unauthorized, ValidationError,
};

/// Re-exports validated extractors, the input derive, and ordered issue contracts.
#[cfg(feature = "http")]
pub use furnace_rs_common::{
    Input, SourcedValidationIssue, ValidatedJson, ValidatedPath, ValidatedQuery, ValidationErrors,
    ValidationIssue, ValidationPathSegment, ValidationResult, ValidationSource,
};

/// Re-exports HTTP router construction, configuration, and runtime startup functions.
#[cfg(feature = "http")]
pub use furnace_rs_common::{
    FURNACE031, FurnaceBurnExt, HttpRuntimeError, build_router, configure_router, serve,
    serve_router,
};

/// Re-exports guarded Passport authentication and policy contracts.
#[cfg(all(feature = "http", feature = "jwt"))]
pub use furnace_rs_common::{
    Authenticated, ClaimsPrincipal, FURNACE130, FURNACE131, PassportContext, PassportError,
    PassportErrorKind, PassportGuard, PassportGuardBuilder, PassportPrincipal, PassportRejection,
    PassportResult, PassportStrategy, TokenSource, VerifiedToken, guard, passport_strategy,
};

/// Re-exports the managed-controller declaration attribute.
#[cfg(feature = "http")]
pub use furnace_rs_common::controller;

/// Re-exports the DELETE route-contract attribute.
#[cfg(feature = "http")]
pub use furnace_rs_common::delete;

/// Re-exports the GET route-contract attribute.
#[cfg(feature = "http")]
pub use furnace_rs_common::get;

/// Re-exports the PATCH route-contract attribute.
#[cfg(feature = "http")]
pub use furnace_rs_common::patch;

/// Re-exports the POST route-contract attribute.
#[cfg(feature = "http")]
pub use furnace_rs_common::post;

/// Re-exports the route-trait declaration attribute.
#[cfg(feature = "http")]
pub use furnace_rs_common::routes;

/// Re-exports the PUT route-contract attribute.
#[cfg(feature = "http")]
pub use furnace_rs_common::put;

/// Re-exports extensions when the `extra` feature is enabled.
#[cfg(feature = "extra")]
pub use furnace_rs_extra as extra;

/// Collects application-facing furnace-rs imports.
pub mod prelude {
    /// Static controller protection declarations.
    #[cfg(feature = "http")]
    pub use furnace_rs_common::{SealRegistration, Sealable};
    /// Re-exports explicit cauldron registration and dependency declarations.
    pub use furnace_rs_core::{Cauldron, CauldronRegistration, burner, cauldron, element, storage};

    /// Re-exports the asynchronous furnace-rs entry-point attribute.
    pub use furnace_rs_core::main;

    /// Re-exports the managed-controller declaration attribute.
    #[cfg(feature = "http")]
    pub use furnace_rs_common::controller;

    /// Re-exports route-contract attributes.
    #[cfg(feature = "http")]
    pub use furnace_rs_common::{delete, get, patch, post, put, routes};

    /// Re-exports standard HTTP request extractors and typed-header support.
    #[cfg(feature = "http")]
    pub use furnace_rs_common::{Header, Json, Path, Query, Request, headers};

    /// Re-exports standard HTTP response types.
    #[cfg(feature = "http")]
    pub use furnace_rs_common::{
        BadRequest, Conflict, Created, Forbidden, HttpError, HttpResult, InternalError, NoContent,
        NotFound, Unauthorized, ValidationError,
    };

    /// Re-exports validated extractors, the input derive, and ordered issue contracts.
    #[cfg(feature = "http")]
    pub use furnace_rs_common::{
        Input, SourcedValidationIssue, ValidatedJson, ValidatedPath, ValidatedQuery,
        ValidationErrors, ValidationIssue, ValidationPathSegment, ValidationResult,
        ValidationSource,
    };

    /// Re-exports HTTP router construction, configuration, and runtime startup functions.
    #[cfg(feature = "http")]
    pub use furnace_rs_common::{
        FURNACE031, FurnaceBurnExt, HttpRuntimeError, build_router, configure_router, serve,
        serve_router,
    };

    /// Re-exports application-facing Passport guards, strategies, and extractors.
    #[cfg(all(feature = "http", feature = "jwt"))]
    pub use furnace_rs_common::{
        Authenticated, ClaimsPrincipal, FURNACE130, FURNACE131, PassportContext, PassportError,
        PassportErrorKind, PassportGuard, PassportGuardBuilder, PassportPrincipal,
        PassportRejection, PassportResult, PassportStrategy, TokenSource, VerifiedToken, guard,
        passport_strategy,
    };

    /// Re-exports the standard logger façade, backend contract, and global module.
    #[cfg(feature = "logger")]
    pub use furnace_rs_common::{
        ConsoleLoggerService, LogLevel, Logger, LoggerCauldron, LoggerService,
    };

    /// Re-exports application-facing Passport JWT contracts and services.
    #[cfg(feature = "jwt")]
    pub use furnace_rs_common::{
        FURNACE120, FURNACE121, JwtAlgorithm, JwtClaims, JwtError, JwtErrorKind, JwtHeader,
        JwtResult, JwtService, JwtSignOptions, JwtTokenKind, JwtValidation, PassportConfig,
        RegisteredJwtClaims, VerifiedJwt,
    };

    /// Re-exports strict cookie extraction, response composition, and time types.
    #[cfg(feature = "cookies")]
    pub use furnace_rs_common::{
        Cookie, CookieError, CookieErrorKind, CookieJar, CookieRejection, CookieResult, Expiration,
        FURNACE110, SameSite, cookie,
    };

    /// Re-exports types used to build, run, and inspect an application.
    pub use furnace_rs_core::{
        ApplicationContext, ApplicationGraph, AutoConfigurationConfigEvidence,
        AutoConfigurationReasonCode, AutoConfigurationReport, AutoConfigurationRequirement,
        AutoConfigurationStatus, Catalog, CauldronGraph, CauldronImportDescriptor,
        CauldronImportEdge, CauldronNode, Config, ConfigBuilder, Configuration,
        ConfigurationErrors, ConfigurationIssue, ConfigurationResult, ConstructionPlan,
        ConstructionStep, DependencyEdge, Diagnostic, Error, Furnace, GraphAnalysis, LifecycleHook,
        LifecycleState, ProviderNode, ProviderOrigin, ProviderOwnership, ProviderState,
        ProviderVisibility, Secret, SourceLocation,
    };
}
