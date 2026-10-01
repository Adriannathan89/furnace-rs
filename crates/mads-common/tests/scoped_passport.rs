//! Context-sensitive Passport strategy selection tests.

#![cfg(all(feature = "http", feature = "jwt"))]
#![allow(missing_docs)]

use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode, header::AUTHORIZATION},
};

use mads_common::{
    ClaimsPrincipal, JwtClaims, JwtService, JwtSignOptions, JwtTokenKind, MADS130, PassportContext,
    PassportPrincipal, PassportResult, PassportStrategy, PassportStrategyCatalog,
    PassportStrategyPreflight, build_router,
    core::{Config, ConfigBuilder, Furnace, Mads, MapSource, Result},
};
use tower::ServiceExt;

static FIRST_STRATEGY_CALLS: AtomicUsize = AtomicUsize::new(0);
static SECOND_STRATEGY_CALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(serde::Deserialize, serde::Serialize)]
pub struct FirstClaims {
    marker: u8,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SecondClaims {
    marker: u8,
}

#[derive(serde::Deserialize)]
pub struct CandidateClaims;

#[derive(serde::Deserialize)]
pub struct NoCustomClaims;

pub struct NonBuiltinPrincipal;

pub struct FirstPrincipal;

pub struct SecondPrincipal;

macro_rules! no_permissions {
    ($principal:ty) => {
        impl PassportPrincipal for $principal {
            fn has_role(&self, _role: &str) -> bool {
                false
            }

            fn has_permission(&self, _permission: &str) -> bool {
                false
            }
        }
    };
}

no_permissions!(FirstClaims);
no_permissions!(SecondClaims);
no_permissions!(CandidateClaims);
no_permissions!(NoCustomClaims);
no_permissions!(NonBuiltinPrincipal);
no_permissions!(FirstPrincipal);
no_permissions!(SecondPrincipal);

mod first {
    use super::*;

    #[mads_core::burner]
    pub struct FirstJwtStrategy;

    #[mads_common::passport_strategy(name = "jwt")]
    impl PassportStrategy for FirstJwtStrategy {
        type Claims = FirstClaims;
        type Principal = FirstPrincipal;

        const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

        async fn validate(
            &self,
            _context: &PassportContext<'_>,
            _claims: &JwtClaims<Self::Claims>,
        ) -> PassportResult<Self::Principal> {
            FIRST_STRATEGY_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(FirstPrincipal)
        }
    }

    #[mads_common::routes]
    #[mads_common::guard(strategy = "jwt", principal = FirstPrincipal)]
    pub trait FirstRoutes {
        #[mads_common::get("/first")]
        async fn profile(&self) -> &'static str;
    }

    #[mads_common::controller(routes = [FirstRoutes])]
    pub struct FirstController;

    impl FirstRoutes for FirstController {
        async fn profile(&self) -> &'static str {
            "first"
        }
    }

    #[mads_common::core::furnace]
    pub struct FirstModule;

    impl mads_common::core::Furnace for FirstModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.provide::<FirstJwtStrategy>()
                .controller::<FirstController>()
                .export::<FirstJwtStrategy>()
        }
    }
}

mod second {
    use super::*;

    #[mads_core::burner]
    pub struct SecondJwtStrategy;

    #[mads_common::passport_strategy(name = "jwt")]
    impl PassportStrategy for SecondJwtStrategy {
        type Claims = SecondClaims;
        type Principal = SecondPrincipal;

        const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

        async fn validate(
            &self,
            _context: &PassportContext<'_>,
            _claims: &JwtClaims<Self::Claims>,
        ) -> PassportResult<Self::Principal> {
            SECOND_STRATEGY_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(SecondPrincipal)
        }
    }

    #[mads_common::routes]
    #[mads_common::guard(strategy = "jwt", principal = SecondPrincipal)]
    pub trait SecondRoutes {
        #[mads_common::get("/second")]
        async fn profile(&self) -> &'static str;
    }

    #[mads_common::controller(routes = [SecondRoutes])]
    pub struct SecondController;

    impl SecondRoutes for SecondController {
        async fn profile(&self) -> &'static str {
            "second"
        }
    }

    #[mads_common::core::furnace]
    pub struct SecondModule;

    impl mads_common::core::Furnace for SecondModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.provide::<SecondJwtStrategy>()
                .controller::<SecondController>()
                .export::<SecondJwtStrategy>()
        }
    }
}

mod candidate_one {
    use super::*;

    #[mads_core::burner]
    pub struct CandidateOneJwtStrategy;

    #[mads_common::passport_strategy(name = "jwt")]
    impl PassportStrategy for CandidateOneJwtStrategy {
        type Claims = CandidateClaims;
        type Principal = ClaimsPrincipal<CandidateClaims>;

        const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

        async fn validate(
            &self,
            _context: &PassportContext<'_>,
            _claims: &JwtClaims<Self::Claims>,
        ) -> PassportResult<Self::Principal> {
            unreachable!("preflight never executes a Passport strategy")
        }
    }

    #[mads_common::core::furnace]
    pub struct CandidateOneModule;

    impl mads_common::core::Furnace for CandidateOneModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.provide::<CandidateOneJwtStrategy>()
                .export::<CandidateOneJwtStrategy>()
        }
    }
}

mod candidate_two {
    use super::*;

    #[mads_core::burner]
    pub struct CandidateTwoJwtStrategy;

    #[mads_common::passport_strategy(name = "jwt")]
    impl PassportStrategy for CandidateTwoJwtStrategy {
        type Claims = CandidateClaims;
        type Principal = ClaimsPrincipal<CandidateClaims>;

        const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

        async fn validate(
            &self,
            _context: &PassportContext<'_>,
            _claims: &JwtClaims<Self::Claims>,
        ) -> PassportResult<Self::Principal> {
            unreachable!("preflight never executes a Passport strategy")
        }
    }

    #[mads_common::core::furnace]
    pub struct CandidateTwoModule;

    impl mads_common::core::Furnace for CandidateTwoModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.provide::<CandidateTwoJwtStrategy>()
                .export::<CandidateTwoJwtStrategy>()
        }
    }
}

mod two_candidates {
    use super::*;

    #[mads_common::routes]
    #[mads_common::guard(strategy = "jwt", principal = ClaimsPrincipal<CandidateClaims>)]
    pub trait CandidateRoutes {
        #[mads_common::get("/candidates")]
        async fn profile(&self) -> &'static str;
    }

    #[mads_common::controller(routes = [CandidateRoutes])]
    pub struct CandidateController;

    impl CandidateRoutes for CandidateController {
        async fn profile(&self) -> &'static str {
            "candidates"
        }
    }

    #[mads_common::core::furnace]
    pub struct CandidateGuardModule;

    impl mads_common::core::Furnace for CandidateGuardModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.controller::<CandidateController>()
                .import(super::candidate_one::CandidateOneModule)
                .import(super::candidate_two::CandidateTwoModule)
        }
    }
}

mod private_strategy {
    use super::*;

    #[mads_core::burner]
    struct PrivateJwtStrategy;

    #[mads_common::passport_strategy(name = "jwt")]
    impl PassportStrategy for PrivateJwtStrategy {
        type Claims = CandidateClaims;
        type Principal = NonBuiltinPrincipal;

        const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

        async fn validate(
            &self,
            _context: &PassportContext<'_>,
            _claims: &JwtClaims<Self::Claims>,
        ) -> PassportResult<Self::Principal> {
            unreachable!("preflight never executes a Passport strategy")
        }
    }

    #[mads_common::core::furnace]
    pub struct PrivateStrategyModule;

    impl mads_common::core::Furnace for PrivateStrategyModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.provide::<PrivateJwtStrategy>()
        }
    }
}

mod private_import {
    use super::*;

    #[mads_common::routes]
    #[mads_common::guard(strategy = "jwt", principal = NonBuiltinPrincipal)]
    pub trait PrivateRoutes {
        #[mads_common::get("/private")]
        async fn profile(&self) -> &'static str;
    }

    #[mads_common::controller(routes = [PrivateRoutes])]
    pub struct PrivateController;

    impl PrivateRoutes for PrivateController {
        async fn profile(&self) -> &'static str {
            "private"
        }
    }

    #[mads_common::core::furnace]
    pub struct PrivateGuardModule;

    impl mads_common::core::Furnace for PrivateGuardModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.controller::<PrivateController>()
                .import(super::private_strategy::PrivateStrategyModule)
        }
    }
}

mod transitive_strategy {
    use super::*;

    #[mads_core::burner]
    pub struct TransitiveJwtStrategy;

    #[mads_common::passport_strategy(name = "jwt")]
    impl PassportStrategy for TransitiveJwtStrategy {
        type Claims = CandidateClaims;
        type Principal = NonBuiltinPrincipal;

        const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

        async fn validate(
            &self,
            _context: &PassportContext<'_>,
            _claims: &JwtClaims<Self::Claims>,
        ) -> PassportResult<Self::Principal> {
            unreachable!("preflight never executes a Passport strategy")
        }
    }

    #[mads_common::core::furnace]
    pub struct TransitiveStrategyModule;

    impl mads_common::core::Furnace for TransitiveStrategyModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.provide::<TransitiveJwtStrategy>()
                .export::<TransitiveJwtStrategy>()
        }
    }
}

mod transitive_import {
    #[mads_common::core::furnace]
    pub struct MiddleModule;

    impl mads_common::core::Furnace for MiddleModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.import(super::transitive_strategy::TransitiveStrategyModule)
        }
    }
}

mod transitive_guard {
    use super::*;

    #[mads_common::routes]
    #[mads_common::guard(strategy = "jwt", principal = NonBuiltinPrincipal)]
    pub trait TransitiveRoutes {
        #[mads_common::get("/transitive")]
        async fn profile(&self) -> &'static str;
    }

    #[mads_common::controller(routes = [TransitiveRoutes])]
    pub struct TransitiveController;

    impl TransitiveRoutes for TransitiveController {
        async fn profile(&self) -> &'static str {
            "transitive"
        }
    }

    #[mads_common::core::furnace]
    pub struct TransitiveGuardModule;

    impl mads_common::core::Furnace for TransitiveGuardModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.controller::<TransitiveController>()
                .import(super::transitive_import::MiddleModule)
        }
    }
}

mod no_custom {
    use super::*;

    #[mads_common::routes]
    #[mads_common::guard(strategy = "jwt", principal = ClaimsPrincipal<NoCustomClaims>)]
    pub trait NoCustomRoutes {
        #[mads_common::get("/builtin")]
        async fn profile(&self) -> &'static str;
    }

    #[mads_common::controller(routes = [NoCustomRoutes])]
    pub struct NoCustomController;

    impl NoCustomRoutes for NoCustomController {
        async fn profile(&self) -> &'static str {
            "builtin"
        }
    }

    #[mads_common::core::furnace]
    pub struct NoCustomModule;

    impl mads_common::core::Furnace for NoCustomModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.controller::<NoCustomController>()
        }
    }
}

mod unimported_nested_strategy {
    use super::*;

    #[mads_common::routes]
    #[mads_common::guard(strategy = "jwt", principal = NonBuiltinPrincipal)]
    pub trait ParentRoutes {
        #[mads_common::get("/nested")]
        async fn profile(&self) -> &'static str;
    }

    #[mads_common::controller(routes = [ParentRoutes])]
    pub struct ParentController;

    impl ParentRoutes for ParentController {
        async fn profile(&self) -> &'static str {
            "nested"
        }
    }

    pub mod child {
        use super::*;

        #[mads_core::burner]
        pub struct ChildJwtStrategy;

        #[mads_common::passport_strategy(name = "jwt")]
        impl PassportStrategy for ChildJwtStrategy {
            type Claims = CandidateClaims;
            type Principal = NonBuiltinPrincipal;

            const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

            async fn validate(
                &self,
                _context: &PassportContext<'_>,
                _claims: &JwtClaims<Self::Claims>,
            ) -> PassportResult<Self::Principal> {
                unreachable!("preflight never executes a Passport strategy")
            }
        }

        #[mads_common::core::furnace]
        pub struct ChildStrategyModule;

        impl mads_common::core::Furnace for ChildStrategyModule {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.provide::<ChildJwtStrategy>()
                    .export::<ChildJwtStrategy>()
            }
        }
    }

    #[mads_common::core::furnace]
    pub struct ParentGuardModule;

    impl mads_common::core::Furnace for ParentGuardModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.controller::<ParentController>()
        }
    }
}

mod roots {
    pub(super) mod first {
        #[mads_common::core::furnace]
        pub struct FirstRoot;

        impl mads_common::core::Furnace for FirstRoot {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.provide::<mads_common::JwtService>()
                    .export::<mads_common::JwtService>()
                    .global()
                    .import(super::super::first::FirstModule)
            }
        }
    }

    pub(super) mod second {
        #[mads_common::core::furnace]
        pub struct SecondRoot;

        impl mads_common::core::Furnace for SecondRoot {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.provide::<mads_common::JwtService>()
                    .export::<mads_common::JwtService>()
                    .global()
                    .import(super::super::second::SecondModule)
            }
        }
    }

    pub(super) mod candidate {
        #[mads_common::core::furnace]
        pub struct CandidateRoot;

        impl mads_common::core::Furnace for CandidateRoot {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.provide::<mads_common::JwtService>()
                    .export::<mads_common::JwtService>()
                    .global()
                    .import(super::super::two_candidates::CandidateGuardModule)
            }
        }
    }

    pub(super) mod private {
        #[mads_common::core::furnace]
        pub struct PrivateRoot;

        impl mads_common::core::Furnace for PrivateRoot {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.provide::<mads_common::JwtService>()
                    .export::<mads_common::JwtService>()
                    .global()
                    .import(super::super::private_import::PrivateGuardModule)
            }
        }
    }

    pub(super) mod transitive {
        #[mads_common::core::furnace]
        pub struct TransitiveRoot;

        impl mads_common::core::Furnace for TransitiveRoot {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.provide::<mads_common::JwtService>()
                    .export::<mads_common::JwtService>()
                    .global()
                    .import(super::super::transitive_guard::TransitiveGuardModule)
            }
        }
    }

    pub(super) mod no_custom {
        #[mads_common::core::furnace]
        pub struct NoCustomRoot;

        impl mads_common::core::Furnace for NoCustomRoot {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.provide::<mads_common::JwtService>()
                    .export::<mads_common::JwtService>()
                    .global()
                    .import(super::super::no_custom::NoCustomModule)
            }
        }
    }

    pub(super) mod nested {
        #[mads_common::core::furnace]
        pub struct NestedRoot;

        impl mads_common::core::Furnace for NestedRoot {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.provide::<mads_common::JwtService>()
                    .export::<mads_common::JwtService>()
                    .global()
                    .import(super::super::unimported_nested_strategy::ParentGuardModule)
            }
        }
    }
}

fn config() -> Config {
    ConfigBuilder::new()
        .source(MapSource::new(
            "mads.toml",
            [("passport.secret", "01234567890123456789012345678901")],
        ))
        .build()
        .unwrap()
}

async fn application_for<M: Furnace>() -> Mads {
    let config = config();
    let jwt = JwtService::from_config(&config).unwrap();
    let mut builder = Mads::builder_with_config(config);
    builder.provide(jwt).unwrap();
    builder.root::<M>().unwrap();
    builder.build().await.unwrap()
}

fn authenticated_request(path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn preflight_for<M: Furnace>() -> Result<PassportStrategyPreflight<'static>> {
    let graph = mads_common::core::__private::build_module_graph::<M>()?;
    mads_common::__private::preflight_scoped(Some(&graph))
}

fn one_visible_custom_overrides_builtin() -> Result<PassportStrategyPreflight<'static>> {
    preflight_for::<roots::first::FirstRoot>()
}

fn two_visible_custom_candidates() -> Result<PassportStrategyPreflight<'static>> {
    preflight_for::<roots::candidate::CandidateRoot>()
}

fn same_name_in_disjoint_contexts() -> Result<()> {
    let first = preflight_for::<roots::first::FirstRoot>()?;
    let second = preflight_for::<roots::second::SecondRoot>()?;

    assert_selected_adapter::<first::FirstJwtStrategy>(&first);
    assert_selected_adapter::<second::SecondJwtStrategy>(&second);
    Ok(())
}

fn private_imported_strategy() -> Result<PassportStrategyPreflight<'static>> {
    preflight_for::<roots::private::PrivateRoot>()
}

fn transitively_imported_strategy() -> Result<PassportStrategyPreflight<'static>> {
    preflight_for::<roots::transitive::TransitiveRoot>()
}

fn strategy_in_an_unimported_child_module() -> Result<PassportStrategyPreflight<'static>> {
    preflight_for::<roots::nested::NestedRoot>()
}

fn assert_selected_adapter<S>(preflight: &PassportStrategyPreflight<'_>)
where
    S: 'static,
{
    let expected = PassportStrategyCatalog::strategies()
        .into_iter()
        .find(|strategy| strategy.provider_type_id() == TypeId::of::<S>())
        .expect("the strategy descriptor must be registered")
        .adapter();
    let selected = preflight.bindings()[0].adapter();
    assert!(std::ptr::fn_addr_eq(selected, expected));
}

#[test]
fn one_visible_custom_overrides_the_builtin_adapter() {
    let preflight = one_visible_custom_overrides_builtin().unwrap();

    assert!(!preflight.bindings()[0].is_builtin());
    assert_selected_adapter::<first::FirstJwtStrategy>(&preflight);
}

#[test]
fn duplicate_custom_strategies_are_rejected_only_when_visible_to_one_guard() {
    assert_eq!(two_visible_custom_candidates().unwrap_err().code(), MADS130);
}

#[test]
fn same_name_in_disjoint_contexts_is_allowed() {
    assert!(same_name_in_disjoint_contexts().is_ok());
}

#[test]
fn private_direct_imported_strategy_is_not_visible() {
    assert_eq!(private_imported_strategy().unwrap_err().code(), MADS130);
}

#[test]
fn transitively_imported_strategy_is_not_visible() {
    assert_eq!(
        transitively_imported_strategy().unwrap_err().code(),
        MADS130
    );
}

#[test]
fn strategy_in_an_unimported_child_module_is_not_visible() {
    assert_eq!(
        strategy_in_an_unimported_child_module().unwrap_err().code(),
        MADS130
    );
}

#[test]
fn no_visible_custom_strategy_retains_the_builtin_jwt_adapter() {
    let preflight = preflight_for::<roots::no_custom::NoCustomRoot>().unwrap();

    assert!(preflight.bindings()[0].is_builtin());
}

#[test]
fn rootless_scoped_preflight_retains_global_duplicate_validation() {
    assert_eq!(
        mads_common::__private::preflight_scoped(None)
            .unwrap_err()
            .code(),
        MADS130
    );
}

#[tokio::test]
async fn request_uses_context_binding() {
    let first_application = application_for::<roots::first::FirstRoot>().await;
    let first_token = first_application
        .context()
        .resolve::<JwtService>()
        .unwrap()
        .sign(
            FirstClaims { marker: 1 },
            JwtSignOptions::access(Duration::from_secs(60)),
        )
        .unwrap();
    FIRST_STRATEGY_CALLS.store(0, Ordering::SeqCst);
    SECOND_STRATEGY_CALLS.store(0, Ordering::SeqCst);

    let first_response = build_router(&first_application)
        .unwrap()
        .oneshot(authenticated_request("/first", &first_token))
        .await
        .unwrap();

    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(FIRST_STRATEGY_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(SECOND_STRATEGY_CALLS.load(Ordering::SeqCst), 0);

    let second_application = application_for::<roots::second::SecondRoot>().await;
    let second_token = second_application
        .context()
        .resolve::<JwtService>()
        .unwrap()
        .sign(
            SecondClaims { marker: 2 },
            JwtSignOptions::access(Duration::from_secs(60)),
        )
        .unwrap();
    FIRST_STRATEGY_CALLS.store(0, Ordering::SeqCst);
    SECOND_STRATEGY_CALLS.store(0, Ordering::SeqCst);

    let second_response = build_router(&second_application)
        .unwrap()
        .oneshot(authenticated_request("/second", &second_token))
        .await
        .unwrap();

    assert_eq!(second_response.status(), StatusCode::OK);
    assert_eq!(FIRST_STRATEGY_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(SECOND_STRATEGY_CALLS.load(Ordering::SeqCst), 1);
}
