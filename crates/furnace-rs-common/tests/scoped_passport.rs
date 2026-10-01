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

use furnace_rs_common::{
    ClaimsPrincipal, FURNACE130, JwtClaims, JwtService, JwtSignOptions, JwtTokenKind,
    PassportContext, PassportPrincipal, PassportResult, PassportStrategy, PassportStrategyCatalog,
    PassportStrategyPreflight, build_router,
    core::{Cauldron, Config, ConfigBuilder, Furnace, MapSource, Result},
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

    #[furnace_rs_core::burner]
    pub struct FirstJwtStrategy;

    #[furnace_rs_common::passport_strategy(name = "jwt")]
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

    #[furnace_rs_common::routes]
    #[furnace_rs_common::guard(strategy = "jwt", principal = FirstPrincipal)]
    pub trait FirstRoutes {
        #[furnace_rs_common::get("/first")]
        async fn profile(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [FirstRoutes])]
    pub struct FirstController;

    impl FirstRoutes for FirstController {
        async fn profile(&self) -> &'static str {
            "first"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct FirstCauldron;

    impl furnace_rs_common::core::Cauldron for FirstCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.provide::<FirstJwtStrategy>()
                .controller::<FirstController>()
                .export::<FirstJwtStrategy>()
        }
    }
}

mod second {
    use super::*;

    #[furnace_rs_core::burner]
    pub struct SecondJwtStrategy;

    #[furnace_rs_common::passport_strategy(name = "jwt")]
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

    #[furnace_rs_common::routes]
    #[furnace_rs_common::guard(strategy = "jwt", principal = SecondPrincipal)]
    pub trait SecondRoutes {
        #[furnace_rs_common::get("/second")]
        async fn profile(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [SecondRoutes])]
    pub struct SecondController;

    impl SecondRoutes for SecondController {
        async fn profile(&self) -> &'static str {
            "second"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct SecondCauldron;

    impl furnace_rs_common::core::Cauldron for SecondCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.provide::<SecondJwtStrategy>()
                .controller::<SecondController>()
                .export::<SecondJwtStrategy>()
        }
    }
}

mod candidate_one {
    use super::*;

    #[furnace_rs_core::burner]
    pub struct CandidateOneJwtStrategy;

    #[furnace_rs_common::passport_strategy(name = "jwt")]
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

    #[furnace_rs_common::core::cauldron]
    pub struct CandidateOneCauldron;

    impl furnace_rs_common::core::Cauldron for CandidateOneCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.provide::<CandidateOneJwtStrategy>()
                .export::<CandidateOneJwtStrategy>()
        }
    }
}

mod candidate_two {
    use super::*;

    #[furnace_rs_core::burner]
    pub struct CandidateTwoJwtStrategy;

    #[furnace_rs_common::passport_strategy(name = "jwt")]
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

    #[furnace_rs_common::core::cauldron]
    pub struct CandidateTwoCauldron;

    impl furnace_rs_common::core::Cauldron for CandidateTwoCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.provide::<CandidateTwoJwtStrategy>()
                .export::<CandidateTwoJwtStrategy>()
        }
    }
}

mod two_candidates {
    use super::*;

    #[furnace_rs_common::routes]
    #[furnace_rs_common::guard(strategy = "jwt", principal = ClaimsPrincipal<CandidateClaims>)]
    pub trait CandidateRoutes {
        #[furnace_rs_common::get("/candidates")]
        async fn profile(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [CandidateRoutes])]
    pub struct CandidateController;

    impl CandidateRoutes for CandidateController {
        async fn profile(&self) -> &'static str {
            "candidates"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct CandidateGuardCauldron;

    impl furnace_rs_common::core::Cauldron for CandidateGuardCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<CandidateController>()
                .import(super::candidate_one::CandidateOneCauldron)
                .import(super::candidate_two::CandidateTwoCauldron)
        }
    }
}

mod private_strategy {
    use super::*;

    #[furnace_rs_core::burner]
    struct PrivateJwtStrategy;

    #[furnace_rs_common::passport_strategy(name = "jwt")]
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

    #[furnace_rs_common::core::cauldron]
    pub struct PrivateStrategyCauldron;

    impl furnace_rs_common::core::Cauldron for PrivateStrategyCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.provide::<PrivateJwtStrategy>()
        }
    }
}

mod private_import {
    use super::*;

    #[furnace_rs_common::routes]
    #[furnace_rs_common::guard(strategy = "jwt", principal = NonBuiltinPrincipal)]
    pub trait PrivateRoutes {
        #[furnace_rs_common::get("/private")]
        async fn profile(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [PrivateRoutes])]
    pub struct PrivateController;

    impl PrivateRoutes for PrivateController {
        async fn profile(&self) -> &'static str {
            "private"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct PrivateGuardCauldron;

    impl furnace_rs_common::core::Cauldron for PrivateGuardCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<PrivateController>()
                .import(super::private_strategy::PrivateStrategyCauldron)
        }
    }
}

mod transitive_strategy {
    use super::*;

    #[furnace_rs_core::burner]
    pub struct TransitiveJwtStrategy;

    #[furnace_rs_common::passport_strategy(name = "jwt")]
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

    #[furnace_rs_common::core::cauldron]
    pub struct TransitiveStrategyCauldron;

    impl furnace_rs_common::core::Cauldron for TransitiveStrategyCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.provide::<TransitiveJwtStrategy>()
                .export::<TransitiveJwtStrategy>()
        }
    }
}

mod transitive_import {
    #[furnace_rs_common::core::cauldron]
    pub struct MiddleCauldron;

    impl furnace_rs_common::core::Cauldron for MiddleCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.import(super::transitive_strategy::TransitiveStrategyCauldron)
        }
    }
}

mod transitive_guard {
    use super::*;

    #[furnace_rs_common::routes]
    #[furnace_rs_common::guard(strategy = "jwt", principal = NonBuiltinPrincipal)]
    pub trait TransitiveRoutes {
        #[furnace_rs_common::get("/transitive")]
        async fn profile(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [TransitiveRoutes])]
    pub struct TransitiveController;

    impl TransitiveRoutes for TransitiveController {
        async fn profile(&self) -> &'static str {
            "transitive"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct TransitiveGuardCauldron;

    impl furnace_rs_common::core::Cauldron for TransitiveGuardCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<TransitiveController>()
                .import(super::transitive_import::MiddleCauldron)
        }
    }
}

mod no_custom {
    use super::*;

    #[furnace_rs_common::routes]
    #[furnace_rs_common::guard(strategy = "jwt", principal = ClaimsPrincipal<NoCustomClaims>)]
    pub trait NoCustomRoutes {
        #[furnace_rs_common::get("/builtin")]
        async fn profile(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [NoCustomRoutes])]
    pub struct NoCustomController;

    impl NoCustomRoutes for NoCustomController {
        async fn profile(&self) -> &'static str {
            "builtin"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct NoCustomCauldron;

    impl furnace_rs_common::core::Cauldron for NoCustomCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<NoCustomController>()
        }
    }
}

mod unimported_nested_strategy {
    use super::*;

    #[furnace_rs_common::routes]
    #[furnace_rs_common::guard(strategy = "jwt", principal = NonBuiltinPrincipal)]
    pub trait ParentRoutes {
        #[furnace_rs_common::get("/nested")]
        async fn profile(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [ParentRoutes])]
    pub struct ParentController;

    impl ParentRoutes for ParentController {
        async fn profile(&self) -> &'static str {
            "nested"
        }
    }

    pub mod child {
        use super::*;

        #[furnace_rs_core::burner]
        pub struct ChildJwtStrategy;

        #[furnace_rs_common::passport_strategy(name = "jwt")]
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

        #[furnace_rs_common::core::cauldron]
        pub struct ChildStrategyCauldron;

        impl furnace_rs_common::core::Cauldron for ChildStrategyCauldron {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.provide::<ChildJwtStrategy>()
                    .export::<ChildJwtStrategy>()
            }
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct ParentGuardCauldron;

    impl furnace_rs_common::core::Cauldron for ParentGuardCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<ParentController>()
        }
    }
}

mod roots {
    pub(super) mod first {
        #[furnace_rs_common::core::cauldron]
        pub struct FirstRoot;

        impl furnace_rs_common::core::Cauldron for FirstRoot {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.provide::<furnace_rs_common::JwtService>()
                    .export::<furnace_rs_common::JwtService>()
                    .global()
                    .import(super::super::first::FirstCauldron)
            }
        }
    }

    pub(super) mod second {
        #[furnace_rs_common::core::cauldron]
        pub struct SecondRoot;

        impl furnace_rs_common::core::Cauldron for SecondRoot {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.provide::<furnace_rs_common::JwtService>()
                    .export::<furnace_rs_common::JwtService>()
                    .global()
                    .import(super::super::second::SecondCauldron)
            }
        }
    }

    pub(super) mod candidate {
        #[furnace_rs_common::core::cauldron]
        pub struct CandidateRoot;

        impl furnace_rs_common::core::Cauldron for CandidateRoot {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.provide::<furnace_rs_common::JwtService>()
                    .export::<furnace_rs_common::JwtService>()
                    .global()
                    .import(super::super::two_candidates::CandidateGuardCauldron)
            }
        }
    }

    pub(super) mod private {
        #[furnace_rs_common::core::cauldron]
        pub struct PrivateRoot;

        impl furnace_rs_common::core::Cauldron for PrivateRoot {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.provide::<furnace_rs_common::JwtService>()
                    .export::<furnace_rs_common::JwtService>()
                    .global()
                    .import(super::super::private_import::PrivateGuardCauldron)
            }
        }
    }

    pub(super) mod transitive {
        #[furnace_rs_common::core::cauldron]
        pub struct TransitiveRoot;

        impl furnace_rs_common::core::Cauldron for TransitiveRoot {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.provide::<furnace_rs_common::JwtService>()
                    .export::<furnace_rs_common::JwtService>()
                    .global()
                    .import(super::super::transitive_guard::TransitiveGuardCauldron)
            }
        }
    }

    pub(super) mod no_custom {
        #[furnace_rs_common::core::cauldron]
        pub struct NoCustomRoot;

        impl furnace_rs_common::core::Cauldron for NoCustomRoot {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.provide::<furnace_rs_common::JwtService>()
                    .export::<furnace_rs_common::JwtService>()
                    .global()
                    .import(super::super::no_custom::NoCustomCauldron)
            }
        }
    }

    pub(super) mod nested {
        #[furnace_rs_common::core::cauldron]
        pub struct NestedRoot;

        impl furnace_rs_common::core::Cauldron for NestedRoot {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.provide::<furnace_rs_common::JwtService>()
                    .export::<furnace_rs_common::JwtService>()
                    .global()
                    .import(super::super::unimported_nested_strategy::ParentGuardCauldron)
            }
        }
    }
}

fn config() -> Config {
    ConfigBuilder::new()
        .source(MapSource::new(
            "furnace.toml",
            [("passport.secret", "01234567890123456789012345678901")],
        ))
        .build()
        .unwrap()
}

async fn application_for<M: Cauldron>() -> Furnace {
    let config = config();
    let jwt = JwtService::from_config(&config).unwrap();
    let mut builder = Furnace::builder_with_config(config);
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

fn preflight_for<M: Cauldron>() -> Result<PassportStrategyPreflight<'static>> {
    let graph = furnace_rs_common::core::__private::build_cauldron_graph::<M>()?;
    furnace_rs_common::__private::preflight_scoped(Some(&graph))
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
    assert_eq!(
        two_visible_custom_candidates().unwrap_err().code(),
        FURNACE130
    );
}

#[test]
fn same_name_in_disjoint_contexts_is_allowed() {
    assert!(same_name_in_disjoint_contexts().is_ok());
}

#[test]
fn private_direct_imported_strategy_is_not_visible() {
    assert_eq!(private_imported_strategy().unwrap_err().code(), FURNACE130);
}

#[test]
fn transitively_imported_strategy_is_not_visible() {
    assert_eq!(
        transitively_imported_strategy().unwrap_err().code(),
        FURNACE130
    );
}

#[test]
fn strategy_in_an_unimported_child_module_is_not_visible() {
    assert_eq!(
        strategy_in_an_unimported_child_module().unwrap_err().code(),
        FURNACE130
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
        furnace_rs_common::__private::preflight_scoped(None)
            .unwrap_err()
            .code(),
        FURNACE130
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
