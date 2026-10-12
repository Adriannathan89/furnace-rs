//! Managed Passport strategy preflight contracts.

#![cfg(all(feature = "http", feature = "jwt"))]

use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};

use furnace_rs_common::{
    FURNACE121, FURNACE130, GuardDescriptor, JwtClaims, JwtTokenKind, PassportContext,
    PassportPrincipal, PassportResult, PassportStrategy, PassportStrategyCatalog, TokenSource,
    core::{Config, ConfigBuilder, Furnace, LifecycleFuture, LifecycleHook, SourceLocation},
};

#[derive(serde::Deserialize)]
struct UserClaims {
    user_id: u64,
}

struct UserPrincipal;

impl PassportPrincipal for UserPrincipal {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

struct RefreshPrincipal;

impl PassportPrincipal for RefreshPrincipal {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

struct MismatchedPrincipal;

impl PassportPrincipal for MismatchedPrincipal {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

#[furnace_rs_core::burner]
struct AccessStrategy;

#[furnace_rs_common::passport_strategy(name = "jwt")]
impl PassportStrategy for AccessStrategy {
    type Claims = UserClaims;
    type Principal = UserPrincipal;

    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _context: &PassportContext<'_>,
        claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        let _ = claims.custom.user_id;
        Ok(UserPrincipal)
    }
}

#[furnace_rs_core::burner]
struct RefreshStrategy;

#[furnace_rs_common::passport_strategy(name = "jwt-refresh")]
impl PassportStrategy for RefreshStrategy {
    type Claims = UserClaims;
    type Principal = RefreshPrincipal;

    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Refresh;

    async fn validate(
        &self,
        _context: &PassportContext<'_>,
        _claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        Ok(RefreshPrincipal)
    }
}

#[furnace_rs_common::controller]
struct AccessRoutesController;

#[furnace_rs_common::guard(principal = UserPrincipal, strategy = "jwt")]
struct AccessRoutesControllerGuard;

impl ::furnace_rs_common::Sealable for AccessRoutesController {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        Self::seal::<AccessRoutesControllerGuard>()
    }
}

#[furnace_rs_common::controller]
impl AccessRoutesController {
    #[furnace_rs_common::get("/access")]

    async fn profile(&self) {
        unreachable!("metadata-only endpoint")
    }
}

#[furnace_rs_common::controller]
struct RefreshRoutesController;

#[furnace_rs_common::guard(principal = RefreshPrincipal, strategy = "jwt-refresh")]
struct RefreshRoutesControllerGuard;

impl ::furnace_rs_common::Sealable for RefreshRoutesController {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        Self::seal::<RefreshRoutesControllerGuard>()
    }
}

#[furnace_rs_common::controller]
impl RefreshRoutesController {
    #[furnace_rs_common::post("/refresh")]

    async fn refresh(&self) {
        unreachable!("metadata-only endpoint")
    }
}

fn mismatched_principal_type_id() -> TypeId {
    TypeId::of::<MismatchedPrincipal>()
}

fn mismatched_principal_type_name() -> &'static str {
    std::any::type_name::<MismatchedPrincipal>()
}

const MISMATCHED_GUARD: GuardDescriptor = GuardDescriptor::new(
    "ManualRoutes",
    "mismatched",
    "jwt",
    Some(mismatched_principal_type_id),
    Some(mismatched_principal_type_name),
    TokenSource::Bearer,
    None,
    None,
    &[],
    SourceLocation::new("tests/passport_preflight.rs", 1, 1),
    None,
)
.with_requirement_subject("ManualRoutes::mismatched");

static ORDINARY_CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);
static LIFECYCLE_STARTS: AtomicUsize = AtomicUsize::new(0);

struct OrdinaryProvider;

fn ordinary_provider() -> OrdinaryProvider {
    ORDINARY_CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
    OrdinaryProvider
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct OrdinaryProviderInjector;
impl furnace_rs_core::Injector<OrdinaryProvider> for OrdinaryProviderInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs_core::Result<OrdinaryProvider> {
        Ok(ordinary_provider())
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_ORDINARY_PROVIDER
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_ORDINARY_PROVIDER : furnace_rs_core :: ProviderDescriptor = furnace_rs_core :: __private :: InjectorMetadata :: < OrdinaryProvider , OrdinaryProviderInjector > :: DESCRIPTOR . with_authored_type_name (stringify ! (OrdinaryProvider)) . with_namespace (module_path ! ()) . with_visibility (furnace_rs_core :: ProviderVisibility :: Private) . with_location (furnace_rs_core :: SourceLocation :: new (file ! () , line ! () , column ! ())) ;
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_ORDINARY_PROVIDER }

struct CountingHook;

impl LifecycleHook for CountingHook {
    fn name(&self) -> &str {
        "preflight-counter"
    }

    fn start<'a>(&'a self, _: &'a furnace_rs_core::ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            LIFECYCLE_STARTS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    fn stop<'a>(&'a self, _: &'a furnace_rs_core::ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async { Ok(()) })
    }
}

fn config() -> Config {
    ConfigBuilder::new()
        .source(furnace_rs_core::MapSource::new(
            "furnace.toml",
            [("passport.secret", "01234567890123456789012345678901")],
        ))
        .build()
        .unwrap()
}

#[test]
fn managed_custom_strategies_override_jwt_and_select_refresh_independently() {
    let guards = [
        <AccessRoutesControllerGuard as furnace_rs_common::GuardPolicy>::descriptor(),
        <RefreshRoutesControllerGuard as furnace_rs_common::GuardPolicy>::descriptor(),
    ];
    let preflight = PassportStrategyCatalog::preflight(&guards).unwrap();
    let access = preflight
        .bindings()
        .iter()
        .find(|binding| {
            binding.guard().requirement_subject() == "AccessRoutesControllerGuard::seal"
        })
        .unwrap();
    let refresh = preflight
        .bindings()
        .iter()
        .find(|binding| {
            binding.guard().requirement_subject() == "RefreshRoutesControllerGuard::seal"
        })
        .unwrap();

    assert_eq!(access.strategy(), "jwt");
    assert_eq!(access.token_kind(), JwtTokenKind::Access);
    assert!(!access.is_builtin());
    assert_eq!(refresh.strategy(), "jwt-refresh");
    assert_eq!(refresh.token_kind(), JwtTokenKind::Refresh);
    assert!(!refresh.is_builtin());

    assert!(Furnace::builder_with_config(config()).analyze().is_valid());
}

#[test]
fn preflight_rejects_a_guard_with_a_different_selected_principal_type() {
    let error = PassportStrategyCatalog::preflight(&[&MISMATCHED_GUARD]).unwrap_err();

    assert_eq!(error.code(), FURNACE130);
    assert!(error.to_string().contains("principal_mismatch"));
    assert!(error.to_string().contains("ManualRoutes::mismatched"));
}

#[tokio::test]
async fn missing_jwt_configuration_fails_before_provider_construction_or_lifecycle() {
    ORDINARY_CONSTRUCTIONS.store(0, Ordering::SeqCst);
    LIFECYCLE_STARTS.store(0, Ordering::SeqCst);
    let mut builder = Furnace::builder();
    builder.lifecycle_hook(CountingHook);

    let error = match builder.build().await {
        Ok(_) => panic!("missing JWT configuration must fail before construction"),
        Err(error) => error,
    };

    assert_eq!(error.code(), FURNACE121);
    assert_eq!(ORDINARY_CONSTRUCTIONS.load(Ordering::SeqCst), 0);
    assert_eq!(LIFECYCLE_STARTS.load(Ordering::SeqCst), 0);
}
