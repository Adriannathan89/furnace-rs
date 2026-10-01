//! Selected HTTP metadata fails before provider/default constructors.
#![cfg(feature = "http")]
#![allow(missing_docs)]
use furnace_rs_common::core::__private::{
    AutoConfigurationApplyContext, AutoConfigurationContext, AutoConfigurationContribution,
    AutoConfigurationDescriptor, AutoConfigurationEvaluation,
};
use furnace_rs_common::core::{
    AutoConfigurationReasonCode, Cauldron, CauldronRegistration, FURNACE008, FURNACE030, Furnace,
    SourceLocation, cauldron, element,
};
use furnace_rs_common::{SealRegistration, Sealable, controller};
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
static PROVIDERS: AtomicUsize = AtomicUsize::new(0);
static DEFAULTS: AtomicUsize = AtomicUsize::new(0);
static SEALS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[derive(Clone)]
struct Resource;
#[derive(Clone)]
struct DefaultResource;
#[element]
fn resource() -> Resource {
    PROVIDERS.fetch_add(1, Ordering::SeqCst);
    Resource
}
fn evaluate(_: &AutoConfigurationContext<'_>) -> AutoConfigurationEvaluation {
    AutoConfigurationEvaluation::active(
        AutoConfigurationReasonCode::new("fixture"),
        "fixture",
        vec![],
        vec![],
    )
}
fn apply(
    _: &AutoConfigurationApplyContext<'_>,
) -> furnace_rs_common::core::Result<AutoConfigurationContribution> {
    DEFAULTS.fetch_add(1, Ordering::SeqCst);
    Ok(AutoConfigurationContribution::new(DefaultResource))
}
furnace_rs_common::core::__private::inventory::submit! {
    AutoConfigurationDescriptor::new("test.http.default", "DefaultResource", TypeId::of::<DefaultResource>, SourceLocation::new(file!(), line!(), column!()), evaluate, apply)
}
#[controller]
struct Conflict {
    _resource: Resource,
    _default: DefaultResource,
}
impl Sealable for Conflict {
    fn seals() -> SealRegistration<Self> {
        SEALS.fetch_add(1, Ordering::SeqCst);
        SealRegistration::new()
    }
}
#[controller(route = "/conflict")]
impl Conflict {
    #[get("/:id")]
    fn first(&self) {}
    #[get("/{name}")]
    fn second(&self) {}
}
#[cauldron]
struct ConflictRoot;
impl Cauldron for ConflictRoot {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<Resource>().controller::<Conflict>()
    }
}
#[controller]
struct Missing;
impl Sealable for Missing {
    fn seals() -> SealRegistration<Self> {
        panic!("missing endpoint metadata must be checked first")
    }
}
#[cauldron]
struct MissingRoot;
impl Cauldron for MissingRoot {
    fn register(self) -> CauldronRegistration<Self> {
        self.controller::<Missing>()
    }
}
#[controller]
struct Duplicate;
impl Sealable for Duplicate {
    fn seals() -> SealRegistration<Self> {
        SealRegistration::new()
    }
}
#[controller(route = "/one")]
impl Duplicate {
    #[get]
    fn one(&self) {}
}
#[controller(route = "/two")]
impl Duplicate {
    #[get]
    fn two(&self) {}
}
#[cauldron]
struct DuplicateRoot;
impl Cauldron for DuplicateRoot {
    fn register(self) -> CauldronRegistration<Self> {
        self.controller::<Duplicate>()
    }
}
#[controller]
struct Valid;
impl Sealable for Valid {
    fn seals() -> SealRegistration<Self> {
        SEALS.fetch_add(1, Ordering::SeqCst);
        SealRegistration::new()
    }
}
#[controller(route = "/valid")]
impl Valid {
    #[get]
    fn valid(&self) {}
}
#[cauldron]
struct ValidRoot;
impl Cauldron for ValidRoot {
    fn register(self) -> CauldronRegistration<Self> {
        self.controller::<Valid>()
    }
}

#[tokio::test]
async fn conflicts_reject_before_any_constructor() {
    let _lock = TEST_LOCK.lock().await;
    PROVIDERS.store(0, Ordering::SeqCst);
    DEFAULTS.store(0, Ordering::SeqCst);
    SEALS.store(0, Ordering::SeqCst);
    let mut builder = Furnace::builder();
    builder.root::<ConflictRoot>().unwrap();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|d| d.code() == FURNACE030)
    );
    assert_eq!(PROVIDERS.load(Ordering::SeqCst), 0);
    assert_eq!(DEFAULTS.load(Ordering::SeqCst), 0);
    assert_eq!(SEALS.load(Ordering::SeqCst), 1);
    assert!(builder.build().await.is_err());
    assert_eq!(PROVIDERS.load(Ordering::SeqCst), 0);
    assert_eq!(DEFAULTS.load(Ordering::SeqCst), 0);
    assert_eq!(SEALS.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn missing_or_duplicate_endpoint_sets_are_metadata_errors() {
    let _lock = TEST_LOCK.lock().await;
    for duplicate in [false, true] {
        let mut builder = Furnace::builder();
        if duplicate {
            builder.root::<DuplicateRoot>().unwrap();
        } else {
            builder.root::<MissingRoot>().unwrap();
        }
        assert!(
            builder
                .analyze()
                .diagnostics()
                .iter()
                .any(|d| d.code() == FURNACE008)
        );
        assert!(builder.build().await.is_err());
    }
}
#[tokio::test]
async fn roots_and_focused_fixtures_ignore_unselected_malformed_controllers() {
    let _lock = TEST_LOCK.lock().await;
    SEALS.store(0, Ordering::SeqCst);
    let mut builder = Furnace::builder();
    builder.root::<ValidRoot>().unwrap();
    assert!(builder.analyze().is_valid());
    assert_eq!(SEALS.load(Ordering::SeqCst), 1);
    let mut builder = Furnace::builder();
    builder.__test_focus::<Valid>().unwrap();
    assert!(builder.analyze().is_valid());
    assert_eq!(SEALS.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn complete_catalog_builder_also_rejects_invalid_endpoint_metadata() {
    let _lock = TEST_LOCK.lock().await;
    assert!(!Furnace::builder().analyze().is_valid());
    assert!(Furnace::builder().build().await.is_err());
}

#[cfg(feature = "jwt")]
mod security {
    use super::*;
    use furnace_rs_common::core::{Config, ConfigBuilder, FURNACE009, MapSource};
    use furnace_rs_common::{
        ClaimsPrincipal, JwtService, PassportConfig, PassportPrincipal, guard,
    };
    static POLICY_SEALS: AtomicUsize = AtomicUsize::new(0);
    #[allow(dead_code)]
    #[derive(serde::Deserialize, PassportPrincipal)]
    struct Claims {
        subject: String,
    }
    #[guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>)]
    struct Policy;
    #[controller]
    struct Protected;
    impl Sealable for Protected {
        fn seals() -> SealRegistration<Self> {
            POLICY_SEALS.fetch_add(1, Ordering::SeqCst);
            Self::seal::<Policy>()
        }
    }
    #[controller(route = "/protected")]
    impl Protected {
        #[get]
        fn endpoint(&self) {}
    }
    #[controller]
    struct MultiSeal;
    impl Sealable for MultiSeal {
        fn seals() -> SealRegistration<Self> {
            Self::seal::<Policy>().seal::<Policy>()
        }
    }
    #[controller(route = "/multi")]
    impl MultiSeal {
        #[get]
        fn endpoint(&self) {}
    }
    #[cauldron]
    struct MultiRoot;
    impl Cauldron for MultiRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<MultiSeal>()
        }
    }
    fn config() -> Config {
        ConfigBuilder::new()
            .source(MapSource::new(
                "fixture",
                [("passport.secret", "01234567890123456789012345678901")],
            ))
            .build()
            .unwrap()
    }
    fn supplied_jwt() -> JwtService {
        JwtService::from_passport_config(PassportConfig::from_config(&config()).unwrap()).unwrap()
    }
    #[element]
    fn jwt() -> JwtService {
        PROVIDERS.fetch_add(1, Ordering::SeqCst);
        supplied_jwt()
    }
    #[cauldron]
    struct PrivateJwt;
    impl Cauldron for PrivateJwt {
        fn register(self) -> CauldronRegistration<Self> {
            self.provide::<JwtService>()
        }
    }
    #[cauldron]
    struct ExportedJwt;
    impl Cauldron for ExportedJwt {
        fn register(self) -> CauldronRegistration<Self> {
            self.provide::<JwtService>().export::<JwtService>()
        }
    }
    #[cauldron]
    struct GlobalJwt;
    impl Cauldron for GlobalJwt {
        fn register(self) -> CauldronRegistration<Self> {
            self.provide::<JwtService>().export::<JwtService>().global()
        }
    }
    #[cauldron]
    struct Bridge;
    impl Cauldron for Bridge {
        fn register(self) -> CauldronRegistration<Self> {
            self.import(GlobalJwt)
        }
    }
    #[cauldron]
    struct PrivateRoot;
    impl Cauldron for PrivateRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<Protected>().import(PrivateJwt)
        }
    }
    #[cauldron]
    struct LocalRoot;
    impl Cauldron for LocalRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<Protected>().provide::<JwtService>()
        }
    }
    #[cauldron]
    struct DirectRoot;
    impl Cauldron for DirectRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<Protected>().import(ExportedJwt)
        }
    }
    #[cauldron]
    struct GlobalRoot;
    impl Cauldron for GlobalRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<Protected>().import(Bridge)
        }
    }
    #[cauldron]
    struct AmbientRoot;
    impl Cauldron for AmbientRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<Protected>()
        }
    }
    #[cauldron]
    struct Left;
    impl Cauldron for Left {
        fn register(self) -> CauldronRegistration<Self> {
            self.import(AmbientRoot)
        }
    }
    #[cauldron]
    struct Right;
    impl Cauldron for Right {
        fn register(self) -> CauldronRegistration<Self> {
            self.import(AmbientRoot)
        }
    }
    #[cauldron]
    struct Diamond;
    impl Cauldron for Diamond {
        fn register(self) -> CauldronRegistration<Self> {
            self.import(Left).import(Right)
        }
    }
    #[tokio::test]
    async fn multiple_seals_fail_before_defaults_and_providers() {
        let _lock = TEST_LOCK.lock().await;
        PROVIDERS.store(0, Ordering::SeqCst);
        DEFAULTS.store(0, Ordering::SeqCst);
        let mut builder = Furnace::builder_with_config(config());
        builder.root::<MultiRoot>().unwrap();
        let analysis = builder.analyze();
        assert_eq!(
            analysis
                .diagnostics()
                .iter()
                .filter(|d| d.code() == FURNACE008)
                .count(),
            1
        );
        assert!(builder.build().await.is_err());
        assert_eq!(PROVIDERS.load(Ordering::SeqCst), 0);
        assert_eq!(DEFAULTS.load(Ordering::SeqCst), 0);
    }
    #[tokio::test]
    async fn supplied_jwt_cannot_bypass_a_private_foreign_owner() {
        let _lock = TEST_LOCK.lock().await;
        PROVIDERS.store(0, Ordering::SeqCst);
        DEFAULTS.store(0, Ordering::SeqCst);
        let mut builder = Furnace::builder_with_config(config());
        builder.root::<PrivateRoot>().unwrap();
        builder.provide(supplied_jwt()).unwrap();
        assert!(
            builder
                .analyze()
                .diagnostics()
                .iter()
                .any(|d| d.code() == FURNACE009)
        );
        assert!(builder.build().await.is_err());
        assert_eq!(PROVIDERS.load(Ordering::SeqCst), 0);
        assert_eq!(DEFAULTS.load(Ordering::SeqCst), 0);
    }
    fn valid<M: Cauldron>() {
        let mut builder = Furnace::builder_with_config(config());
        builder.root::<M>().unwrap();
        let analysis = builder.analyze();
        assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    }
    #[tokio::test]
    async fn seals_follow_controller_visibility_and_are_evaluated_once_per_analysis() {
        let _lock = TEST_LOCK.lock().await;
        POLICY_SEALS.store(0, Ordering::SeqCst);
        valid::<LocalRoot>();
        valid::<DirectRoot>();
        valid::<GlobalRoot>();
        valid::<AmbientRoot>();
        valid::<Diamond>();
        assert_eq!(POLICY_SEALS.load(Ordering::SeqCst), 5);
        let mut builder = Furnace::builder_with_config(config());
        builder.__test_focus::<Protected>().unwrap();
        let analysis = builder.analyze();
        assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
        assert_eq!(POLICY_SEALS.load(Ordering::SeqCst), 6);
        assert!(analysis.graph().provider::<JwtService>().is_some());
    }
    #[tokio::test]
    async fn focused_seals_reject_a_missing_required_jwt_supply_even_without_configuration() {
        let _lock = TEST_LOCK.lock().await;
        let mut builder = Furnace::builder();
        builder.__test_focus::<Protected>().unwrap();
        builder.__test_require_provided::<JwtService>();
        assert!(!builder.analyze().is_valid());
        assert!(builder.build().await.is_err());
    }

    #[controller]
    struct DisabledController;
    impl Sealable for DisabledController {
        fn seals() -> SealRegistration<Self> {
            Self::seal::<Policy>()
        }
    }
    #[controller(route = "/disabled")]
    impl DisabledController {
        #[cfg(any())]
        #[get]
        fn endpoint(&self) {}
    }
    #[cauldron]
    struct DisabledRoot;
    impl Cauldron for DisabledRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<DisabledController>()
        }
    }
    #[tokio::test]
    async fn cfg_disabled_endpoints_do_not_require_jwt_configuration() {
        let _lock = TEST_LOCK.lock().await;
        let mut builder = Furnace::builder();
        builder.root::<DisabledRoot>().unwrap();
        let analysis = builder.analyze();
        assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    }

    #[guard(strategy = "custom", principal = Claims)]
    struct CustomPolicy;
    #[controller]
    struct CustomController;
    impl Sealable for CustomController {
        fn seals() -> SealRegistration<Self> {
            Self::seal::<CustomPolicy>()
        }
    }
    #[controller(route = "/custom")]
    impl CustomController {
        #[get]
        fn endpoint(&self) {}
    }
    #[furnace_rs_common::core::burner]
    struct Strategy {
        _resource: Resource,
    }
    #[furnace_rs_common::passport_strategy(name = "custom")]
    impl furnace_rs_common::PassportStrategy for Strategy {
        type Claims = Claims;
        type Principal = Claims;
        const TOKEN_KIND: furnace_rs_common::JwtTokenKind = furnace_rs_common::JwtTokenKind::Access;
        async fn validate(
            &self,
            _: &furnace_rs_common::PassportContext<'_>,
            _: &furnace_rs_common::JwtClaims<Claims>,
        ) -> furnace_rs_common::PassportResult<Claims> {
            unreachable!("metadata only")
        }
    }
    #[cauldron]
    struct PrivateStrategy;
    impl Cauldron for PrivateStrategy {
        fn register(self) -> CauldronRegistration<Self> {
            self.provide::<Resource>().provide::<Strategy>()
        }
    }
    #[cauldron]
    struct ExportedStrategy;
    impl Cauldron for ExportedStrategy {
        fn register(self) -> CauldronRegistration<Self> {
            self.provide::<Resource>()
                .provide::<Strategy>()
                .export::<Strategy>()
        }
    }
    #[cauldron]
    struct GlobalStrategy;
    impl Cauldron for GlobalStrategy {
        fn register(self) -> CauldronRegistration<Self> {
            self.provide::<Resource>()
                .provide::<Strategy>()
                .export::<Strategy>()
                .global()
        }
    }
    #[cauldron]
    struct StrategyBridge;
    impl Cauldron for StrategyBridge {
        fn register(self) -> CauldronRegistration<Self> {
            self.import(GlobalStrategy)
        }
    }
    #[cauldron]
    struct CustomPrivate;
    impl Cauldron for CustomPrivate {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<CustomController>()
                .import(PrivateStrategy)
        }
    }
    #[cauldron]
    struct CustomLocal;
    impl Cauldron for CustomLocal {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<CustomController>()
                .provide::<Resource>()
                .provide::<Strategy>()
        }
    }
    #[cauldron]
    struct CustomDirect;
    impl Cauldron for CustomDirect {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<CustomController>()
                .import(ExportedStrategy)
        }
    }
    #[cauldron]
    struct CustomGlobal;
    impl Cauldron for CustomGlobal {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<CustomController>().import(StrategyBridge)
        }
    }
    struct UnmanagedStrategy;
    #[furnace_rs_common::passport_strategy(name = "unregistered_bad")]
    impl furnace_rs_common::PassportStrategy for UnmanagedStrategy {
        type Claims = Claims;
        type Principal = Claims;
        const TOKEN_KIND: furnace_rs_common::JwtTokenKind = furnace_rs_common::JwtTokenKind::Access;
        async fn validate(
            &self,
            _: &furnace_rs_common::PassportContext<'_>,
            _: &furnace_rs_common::JwtClaims<Claims>,
        ) -> furnace_rs_common::PassportResult<Claims> {
            unreachable!("metadata only")
        }
    }
    #[cauldron]
    struct InvalidStrategyRoot;
    impl Cauldron for InvalidStrategyRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<Valid>().provide::<UnmanagedStrategy>()
        }
    }
    #[tokio::test]
    async fn custom_strategy_visibility_is_checked_from_the_controller_owner() {
        let _lock = TEST_LOCK.lock().await;
        PROVIDERS.store(0, Ordering::SeqCst);
        DEFAULTS.store(0, Ordering::SeqCst);
        let mut builder = Furnace::builder_with_config(config());
        builder.root::<CustomPrivate>().unwrap();
        assert!(
            builder
                .analyze()
                .diagnostics()
                .iter()
                .any(|d| d.code() == furnace_rs_common::FURNACE130)
        );
        assert!(builder.build().await.is_err());
        assert_eq!(PROVIDERS.load(Ordering::SeqCst), 0);
        assert_eq!(DEFAULTS.load(Ordering::SeqCst), 0);
        valid::<CustomLocal>();
        valid::<CustomDirect>();
        valid::<CustomGlobal>();
        let mut builder = Furnace::builder_with_config(config());
        builder.root::<InvalidStrategyRoot>().unwrap();
        assert!(
            builder
                .analyze()
                .diagnostics()
                .iter()
                .any(|d| d.code() == furnace_rs_common::FURNACE130)
        );
    }
    #[tokio::test]
    async fn focused_custom_seals_require_the_selected_strategy_output() {
        let _lock = TEST_LOCK.lock().await;
        PROVIDERS.store(0, Ordering::SeqCst);
        DEFAULTS.store(0, Ordering::SeqCst);
        let mut builder = Furnace::builder_with_config(config());
        builder.__test_focus::<CustomController>().unwrap();
        let analysis = builder.analyze();
        assert!(
            analysis
                .diagnostics()
                .iter()
                .any(|d| d.code() == furnace_rs_common::FURNACE130)
        );
        assert!(builder.build().await.is_err());
        assert_eq!(PROVIDERS.load(Ordering::SeqCst), 0);
        assert_eq!(DEFAULTS.load(Ordering::SeqCst), 0);
    }
}

struct UndeclaredCauldron;
impl Cauldron for UndeclaredCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        CauldronRegistration::new(self)
    }
}
#[cauldron]
struct InvalidGraphRoot;
impl Cauldron for InvalidGraphRoot {
    fn register(self) -> CauldronRegistration<Self> {
        self.import(UndeclaredCauldron)
    }
}
#[tokio::test]
async fn failed_root_selection_does_not_validate_the_complete_inventory() {
    let _lock = TEST_LOCK.lock().await;
    let mut builder = Furnace::builder();
    builder.root::<InvalidGraphRoot>().unwrap();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    assert_eq!(analysis.diagnostics().len(), 1);
    assert_eq!(analysis.diagnostics()[0].code(), FURNACE008);
}
