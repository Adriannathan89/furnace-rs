//! Typed Passport principals, request context, and normalized failures.
//!
//! Passport is available only when both the `http` and `jwt` features are
//! enabled. Guards install [`Authenticated`] principals and [`VerifiedToken`]
//! values into request extensions for typed handler extraction.
//!
//! A managed access strategy receives verified claims and returns the current
//! application identity. A refresh strategy is application-defined and selects
//! the refresh token profile:
//!
//! ```no_run
//! use furnace_rs_common::{
//!     JwtClaims, JwtTokenKind, PassportContext, PassportPrincipal,
//!     PassportResult, PassportStrategy, passport_strategy,
//! };
//! # use furnace_rs_common::{
//! #     core, ErasedAuthentication, JwtService, JwtValidation, PassportError,
//! #     PassportStrategyDescriptor, PassportStrategyFuture,
//! # };
//!
//! #[derive(Clone, serde::Deserialize)]
//! struct UserClaims { user_id: u64 }
//! struct UserPrincipal(u64);
//! impl PassportPrincipal for UserPrincipal {
//!     fn has_role(&self, role: &str) -> bool { role == "user" }
//!     fn has_permission(&self, permission: &str) -> bool {
//!         permission == "profile:read"
//!     }
//! }
//!
//! #[furnace_rs_core::burner]
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
//! #[furnace_rs_core::burner]
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
//! ```
//!
//! Route policy inherits field by field. A method replaces only the fields it
//! supplies, while `skip` removes an inherited guard:
//!
//! ```no_run
//! use furnace_rs_common::{Authenticated, PassportPrincipal, guard, routes};
//! # use furnace_rs_common::{
//! #     __private, core, ErasedAuthentication, GuardDescriptor, GuardPredicate,
//! #     HttpMethod, PolicyClause, PolicyMode, RouteDescriptor, TokenSource,
//! # };
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
//!     #[furnace_rs_common::get("/profile")]
//!     #[guard(
//!         permissions(all = ["profile:read"]),
//!         predicate = owns_profile,
//!     )]
//!     async fn profile(&self, principal: Authenticated<UserPrincipal>);
//!
//!     #[furnace_rs_common::post("/login")]
//!     #[guard(skip)]
//!     async fn login(&self);
//! }
//! # fn main() {}
//! ```
//!
//! With the `cookies` feature, use `source = cookie("refresh_token")`. One
//! guard reads exactly one source and never falls back to Bearer. Roles,
//! permissions, and every synchronous `fn(&Principal) -> bool` predicate are
//! separate AND clauses.

mod context;
mod error;
mod guard;
mod principal;
mod strategy;

pub use context::PassportContext;
#[cfg(feature = "cookies")]
pub use context::PassportCookies;
pub use error::{PassportError, PassportErrorKind, PassportRejection, PassportResult};
pub use guard::{
    BuiltinGuardAdapter, GuardCatalog, GuardDescriptor, GuardPolicy, GuardPredicate,
    GuardPredicateAdapter, NativePassportGuardService, PassportGuard, PassportGuardBuilder,
    PassportGuardLayer, PassportGuardService, PassportGuardState, PolicyClause, PolicyMode,
    TokenSource,
};
pub use principal::{Authenticated, ClaimsPrincipal, PassportPrincipal, VerifiedToken};
pub use strategy::{
    ErasedAuthentication, PassportStrategy, PassportStrategyAdapter, PassportStrategyBinding,
    PassportStrategyCatalog, PassportStrategyDescriptor, PassportStrategyFuture,
    PassportStrategyPreflight,
};

/// Passport strategy registration, resolution, or type mismatch.
pub const FURNACE130: furnace_rs_core::DiagnosticCode =
    furnace_rs_core::DiagnosticCode::new("FURNACE130");

/// Guard metadata or authentication-policy failure.
pub const FURNACE131: furnace_rs_core::DiagnosticCode =
    furnace_rs_core::DiagnosticCode::new("FURNACE131");
