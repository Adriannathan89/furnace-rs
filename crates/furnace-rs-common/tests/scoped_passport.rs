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
    core::{__private::ModuleContract, Config, ConfigBuilder, Furnace, MapSource, Result},
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

    #[furnace_rs_common::controller]
    pub struct FirstController;

    #[furnace_rs_common::guard(principal = FirstPrincipal, strategy = "jwt")]
    struct FirstControllerGuard;

    impl ::furnace_rs_common::Sealable for FirstController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            Self::seal::<FirstControllerGuard>()
        }
    }

    #[furnace_rs_common::controller]
    impl FirstController {
        #[furnace_rs_common::get("/first")]
        async fn profile(&self) -> &'static str {
            "first"
        }
    }

    #[furnace_rs_common::core::cauldron(providers = [FirstJwtStrategy], controllers = [FirstController], exports = [FirstJwtStrategy])]
    pub struct FirstCauldron;
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

    #[furnace_rs_common::controller]
    pub struct SecondController;

    #[furnace_rs_common::guard(principal = SecondPrincipal, strategy = "jwt")]
    struct SecondControllerGuard;

    impl ::furnace_rs_common::Sealable for SecondController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            Self::seal::<SecondControllerGuard>()
        }
    }

    #[furnace_rs_common::controller]
    impl SecondController {
        #[furnace_rs_common::get("/second")]
        async fn profile(&self) -> &'static str {
            "second"
        }
    }

    #[furnace_rs_common::core::cauldron(providers = [SecondJwtStrategy], controllers = [SecondController], exports = [SecondJwtStrategy])]
    pub struct SecondCauldron;
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

    #[furnace_rs_common::core::cauldron(providers = [CandidateOneJwtStrategy], exports = [CandidateOneJwtStrategy])]
    pub struct CandidateOneCauldron;
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

    #[furnace_rs_common::core::cauldron(providers = [CandidateTwoJwtStrategy], exports = [CandidateTwoJwtStrategy])]
    pub struct CandidateTwoCauldron;
}

mod two_candidates {
    use super::*;

    #[furnace_rs_common::controller]
    pub struct CandidateController;

    #[furnace_rs_common::guard(principal = ClaimsPrincipal < CandidateClaims >, strategy = "jwt")]
    struct CandidateControllerGuard;

    impl ::furnace_rs_common::Sealable for CandidateController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            Self::seal::<CandidateControllerGuard>()
        }
    }

    #[furnace_rs_common::controller]
    impl CandidateController {
        #[furnace_rs_common::get("/candidates")]
        async fn profile(&self) -> &'static str {
            "candidates"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [CandidateController], imports = [super::candidate_one::CandidateOneCauldron, super::candidate_two::CandidateTwoCauldron])]
    pub struct CandidateGuardCauldron;
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

    #[furnace_rs_common::core::cauldron(providers = [PrivateJwtStrategy])]
    pub struct PrivateStrategyCauldron;
}

mod private_import {
    use super::*;

    #[furnace_rs_common::controller]
    pub struct PrivateController;

    #[furnace_rs_common::guard(principal = NonBuiltinPrincipal, strategy = "jwt")]
    struct PrivateControllerGuard;

    impl ::furnace_rs_common::Sealable for PrivateController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            Self::seal::<PrivateControllerGuard>()
        }
    }

    #[furnace_rs_common::controller]
    impl PrivateController {
        #[furnace_rs_common::get("/private")]
        async fn profile(&self) -> &'static str {
            "private"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [PrivateController], imports = [super::private_strategy::PrivateStrategyCauldron])]
    pub struct PrivateGuardCauldron;
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

    #[furnace_rs_common::core::cauldron(providers = [TransitiveJwtStrategy], exports = [TransitiveJwtStrategy])]
    pub struct TransitiveStrategyCauldron;
}

mod transitive_import {
    #[furnace_rs_common::core::cauldron(imports = [super::transitive_strategy::TransitiveStrategyCauldron])]
    pub struct MiddleCauldron;
}

mod transitive_guard {
    use super::*;

    #[furnace_rs_common::controller]
    pub struct TransitiveController;

    #[furnace_rs_common::guard(principal = NonBuiltinPrincipal, strategy = "jwt")]
    struct TransitiveControllerGuard;

    impl ::furnace_rs_common::Sealable for TransitiveController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            Self::seal::<TransitiveControllerGuard>()
        }
    }

    #[furnace_rs_common::controller]
    impl TransitiveController {
        #[furnace_rs_common::get("/transitive")]
        async fn profile(&self) -> &'static str {
            "transitive"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [TransitiveController], imports = [super::transitive_import::MiddleCauldron])]
    pub struct TransitiveGuardCauldron;
}

mod no_custom {
    use super::*;

    #[furnace_rs_common::controller]
    pub struct NoCustomController;

    #[furnace_rs_common::guard(principal = ClaimsPrincipal < NoCustomClaims >, strategy = "jwt")]
    struct NoCustomControllerGuard;

    impl ::furnace_rs_common::Sealable for NoCustomController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            Self::seal::<NoCustomControllerGuard>()
        }
    }

    #[furnace_rs_common::controller]
    impl NoCustomController {
        #[furnace_rs_common::get("/builtin")]
        async fn profile(&self) -> &'static str {
            "builtin"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [NoCustomController])]
    pub struct NoCustomCauldron;
}

mod unimported_nested_strategy {
    use super::*;

    #[furnace_rs_common::controller]
    pub struct ParentController;

    #[furnace_rs_common::guard(principal = NonBuiltinPrincipal, strategy = "jwt")]
    struct ParentControllerGuard;

    impl ::furnace_rs_common::Sealable for ParentController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            Self::seal::<ParentControllerGuard>()
        }
    }

    #[furnace_rs_common::controller]
    impl ParentController {
        #[furnace_rs_common::get("/nested")]
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

        #[furnace_rs_common::core::cauldron(providers = [ChildJwtStrategy], exports = [ChildJwtStrategy])]
        pub struct ChildStrategyCauldron;
    }

    #[furnace_rs_common::core::cauldron(controllers = [ParentController])]
    pub struct ParentGuardCauldron;
}

mod roots {
    pub(super) mod first {
        #[furnace_rs_common::core::cauldron(providers = [furnace_rs_common::JwtService], exports = [furnace_rs_common::JwtService], imports = [super::super::first::FirstCauldron], global)]
        pub struct FirstRoot;
    }

    pub(super) mod second {
        #[furnace_rs_common::core::cauldron(providers = [furnace_rs_common::JwtService], exports = [furnace_rs_common::JwtService], imports = [super::super::second::SecondCauldron], global)]
        pub struct SecondRoot;
    }

    pub(super) mod candidate {
        #[furnace_rs_common::core::cauldron(providers = [furnace_rs_common::JwtService], exports = [furnace_rs_common::JwtService], imports = [super::super::two_candidates::CandidateGuardCauldron], global)]
        pub struct CandidateRoot;
    }

    pub(super) mod private {
        #[furnace_rs_common::core::cauldron(providers = [furnace_rs_common::JwtService], exports = [furnace_rs_common::JwtService], imports = [super::super::private_import::PrivateGuardCauldron], global)]
        pub struct PrivateRoot;
    }

    pub(super) mod transitive {
        #[furnace_rs_common::core::cauldron(providers = [furnace_rs_common::JwtService], exports = [furnace_rs_common::JwtService], imports = [super::super::transitive_guard::TransitiveGuardCauldron], global)]
        pub struct TransitiveRoot;
    }

    pub(super) mod no_custom {
        #[furnace_rs_common::core::cauldron(providers = [furnace_rs_common::JwtService], exports = [furnace_rs_common::JwtService], imports = [super::super::no_custom::NoCustomCauldron], global)]
        pub struct NoCustomRoot;
    }

    pub(super) mod nested {
        #[furnace_rs_common::core::cauldron(providers = [furnace_rs_common::JwtService], exports = [furnace_rs_common::JwtService], imports = [super::super::unimported_nested_strategy::ParentGuardCauldron], global)]
        pub struct NestedRoot;
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

async fn application_for<M: ModuleContract>() -> Furnace {
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

fn preflight_for<M: ModuleContract>() -> Result<PassportStrategyPreflight<'static>> {
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
