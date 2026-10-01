//! Integration tests for rooted provider selection and module boundaries.

use std::any::TypeId;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use mads_core::{
    Config, ConstructionContext, ErasedProvider, Furnace, GraphAnalysis, MADS001, MADS002, MADS009,
    Mads, ProviderDescriptor, ProviderFuture, ProviderKind, ProviderRegistry, ProviderVisibility,
    SourceLocation,
};

mod selected_scope {
    pub mod app {
        use super::unowned::RequiredUnownedUseCase;

        #[mads_core::furnace]
        pub struct AppModule;

        impl mads_core::Furnace for AppModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<RequiredUnownedUseCase>()
                    .provide::<ReachableController>()
                    .provide::<UnusedButOwnedService>()
                    .export::<ReachableController>()
                    .export::<UnusedButOwnedService>()
            }
        }

        #[derive(Clone)]
        pub struct ReachableController;

        #[derive(Clone)]
        pub struct UnusedButOwnedService;

        pub mod providers {
            use super::{ReachableController, RequiredUnownedUseCase, UnusedButOwnedService};

            #[mads_core::element]
            pub fn reachable_controller(_use_case: RequiredUnownedUseCase) -> ReachableController {
                ReachableController
            }

            #[mads_core::element]
            pub fn unused_but_owned_service() -> UnusedButOwnedService {
                UnusedButOwnedService
            }
        }
    }

    pub mod unowned {
        #[derive(Clone)]
        pub struct RequiredUnownedUseCase;

        #[mads_core::element]
        pub fn required_unowned_use_case() -> RequiredUnownedUseCase {
            RequiredUnownedUseCase
        }
    }

    pub mod unreachable {
        #[mads_core::furnace]
        pub struct UnreachableModule;

        impl mads_core::Furnace for UnreachableModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<UnreachableService>()
                    .export::<UnreachableService>()
            }
        }

        #[derive(Clone)]
        pub struct UnreachableService;

        #[mads_core::element]
        pub fn unreachable_service() -> UnreachableService {
            UnreachableService
        }
    }
}

mod direct_public {
    pub mod target {
        #[mads_core::furnace]
        pub struct TargetModule;

        impl mads_core::Furnace for TargetModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<PublicProvider>().export::<PublicProvider>()
            }
        }

        #[derive(Clone)]
        pub struct PublicProvider;

        #[mads_core::element]
        pub fn public_provider() -> PublicProvider {
            PublicProvider
        }
    }

    pub mod root {
        use super::target::{PublicProvider, TargetModule};

        #[mads_core::furnace]
        pub struct DirectRoot;

        impl mads_core::Furnace for DirectRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<DirectConsumer>()
                    .import(TargetModule)
                    .export::<DirectConsumer>()
            }
        }

        #[derive(Clone)]
        pub struct DirectConsumer;

        #[mads_core::element]
        pub fn direct_consumer(_provider: PublicProvider) -> DirectConsumer {
            DirectConsumer
        }
    }
}

mod global_public {
    pub mod database {
        #[mads_core::furnace]
        pub struct DatabaseModule;

        impl mads_core::Furnace for DatabaseModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<DatabasePool>()
                    .export::<DatabasePool>()
                    .global()
            }
        }

        #[derive(Clone)]
        pub struct DatabasePool;

        #[mads_core::element]
        pub fn database_pool() -> DatabasePool {
            DatabasePool
        }
    }

    pub mod user {
        use super::database::DatabasePool;

        #[mads_core::furnace]
        pub struct UserModule;

        impl mads_core::Furnace for UserModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<UserService>().export::<UserService>()
            }
        }

        #[derive(Clone)]
        pub struct UserService;

        #[mads_core::element]
        pub fn user_service(_pool: DatabasePool) -> UserService {
            UserService
        }
    }

    pub mod app {
        use super::{database::DatabaseModule, user::UserModule};

        #[mads_core::furnace]
        pub struct GlobalRoot;

        impl mads_core::Furnace for GlobalRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(DatabaseModule).import(UserModule)
            }
        }
    }
}

mod missing_import {
    pub mod target {
        #[mads_core::furnace]
        pub struct TargetModule;

        impl mads_core::Furnace for TargetModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<PublicProvider>().export::<PublicProvider>()
            }
        }

        #[derive(Clone)]
        pub struct PublicProvider;

        #[mads_core::element]
        pub fn public_provider() -> PublicProvider {
            PublicProvider
        }
    }

    pub mod root {
        use super::target::PublicProvider;

        #[mads_core::furnace]
        pub struct MissingImportRoot;

        impl mads_core::Furnace for MissingImportRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<MissingImportConsumer>()
                    .export::<MissingImportConsumer>()
            }
        }

        #[derive(Clone)]
        pub struct MissingImportConsumer;

        #[mads_core::element]
        pub fn missing_import_consumer(_provider: PublicProvider) -> MissingImportConsumer {
            MissingImportConsumer
        }
    }
}

mod transitive_only {
    pub mod third {
        #[mads_core::furnace]
        pub struct ThirdModule;

        impl mads_core::Furnace for ThirdModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<ThirdProvider>().export::<ThirdProvider>()
            }
        }

        #[derive(Clone)]
        pub struct ThirdProvider;

        #[mads_core::element]
        pub fn third_provider() -> ThirdProvider {
            ThirdProvider
        }
    }

    pub mod middle {
        use super::third::ThirdModule;

        #[mads_core::furnace]
        pub struct MiddleModule;

        impl mads_core::Furnace for MiddleModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(ThirdModule)
            }
        }
    }

    pub mod root {
        use super::{middle::MiddleModule, third::ThirdProvider};

        #[mads_core::furnace]
        pub struct TransitiveRoot;

        impl mads_core::Furnace for TransitiveRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<TransitiveConsumer>()
                    .import(MiddleModule)
                    .export::<TransitiveConsumer>()
            }
        }

        #[derive(Clone)]
        pub struct TransitiveConsumer;

        #[mads_core::element]
        pub fn transitive_consumer(_provider: ThirdProvider) -> TransitiveConsumer {
            TransitiveConsumer
        }
    }
}

mod restricted_crossing {
    pub mod target {
        #[mads_core::furnace]
        pub struct RestrictedTargetModule;

        impl mads_core::Furnace for RestrictedTargetModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<RestrictedProvider>()
            }
        }

        #[derive(Clone)]
        pub(crate) struct RestrictedProvider;

        #[mads_core::element]
        pub(crate) fn restricted_provider() -> RestrictedProvider {
            RestrictedProvider
        }
    }

    pub mod root {
        use super::target::{RestrictedProvider, RestrictedTargetModule};

        #[mads_core::furnace]
        pub struct RestrictedRoot;

        impl mads_core::Furnace for RestrictedRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<RestrictedConsumer>()
                    .import(RestrictedTargetModule)
                    .export::<RestrictedConsumer>()
            }
        }

        #[derive(Clone)]
        pub struct RestrictedConsumer;

        #[mads_core::element]
        pub fn restricted_consumer(_provider: RestrictedProvider) -> RestrictedConsumer {
            RestrictedConsumer
        }
    }
}

mod unowned_bridge {
    pub mod target {
        #[mads_core::furnace]
        pub struct BridgeTargetModule;

        impl mads_core::Furnace for BridgeTargetModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<OwnedTarget>().export::<OwnedTarget>()
            }
        }

        #[derive(Clone)]
        pub struct OwnedTarget;

        #[mads_core::element]
        pub fn owned_target() -> OwnedTarget {
            OwnedTarget
        }
    }

    pub mod bridge {
        use super::target::OwnedTarget;

        #[derive(Clone)]
        pub struct UnownedBridge;

        #[mads_core::element]
        pub fn unowned_bridge(_target: OwnedTarget) -> UnownedBridge {
            UnownedBridge
        }
    }

    pub mod root {
        use super::bridge::UnownedBridge;

        #[mads_core::furnace]
        pub struct BridgeRoot;

        impl mads_core::Furnace for BridgeRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<BridgeConsumer>().export::<BridgeConsumer>()
            }
        }

        #[derive(Clone)]
        pub struct BridgeConsumer;

        #[mads_core::element]
        pub fn bridge_consumer(_bridge: UnownedBridge) -> BridgeConsumer {
            BridgeConsumer
        }
    }
}

mod diamond {
    use super::{AtomicUsize, Ordering};

    pub static CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);

    pub mod shared {
        use super::{CONSTRUCTIONS, Ordering};

        #[mads_core::furnace]
        pub struct SharedModule;

        impl mads_core::Furnace for SharedModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<SharedProvider>().export::<SharedProvider>()
            }
        }

        pub struct SharedProvider;

        #[mads_core::element]
        pub fn shared_provider() -> SharedProvider {
            CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
            SharedProvider
        }
    }

    pub mod left {
        use super::shared::SharedModule;

        #[mads_core::furnace]
        pub struct LeftModule;

        impl mads_core::Furnace for LeftModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(SharedModule)
            }
        }
    }

    pub mod right {
        use super::shared::SharedModule;

        #[mads_core::furnace]
        pub struct RightModule;

        impl mads_core::Furnace for RightModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(SharedModule)
            }
        }
    }

    pub mod root {
        use super::{left::LeftModule, right::RightModule};

        #[mads_core::furnace]
        pub struct DiamondRoot;

        impl mads_core::Furnace for DiamondRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(LeftModule).import(RightModule)
            }
        }
    }
}

mod duplicate_reachable {
    use super::{
        Arc, ConstructionContext, ErasedProvider, ProviderDescriptor, ProviderFuture, ProviderKind,
        ProviderVisibility, SourceLocation, TypeId,
    };

    #[mads_core::furnace]
    pub struct DuplicateRoot;

    impl mads_core::Furnace for DuplicateRoot {
        fn register(self) -> mads_core::FurnaceRegistration<Self> {
            self.provide::<DuplicateProvider>()
        }
    }

    pub struct DuplicateProvider;

    fn duplicate_type_id() -> TypeId {
        TypeId::of::<DuplicateProvider>()
    }

    fn duplicate_constructor<'a>(_: &'a ConstructionContext<'a>) -> ProviderFuture<'a> {
        Box::pin(async { Ok(Arc::new(DuplicateProvider) as ErasedProvider) })
    }

    mads_core::__private::inventory::submit! {
        ProviderDescriptor::new(
            ProviderKind::Provider,
            "DuplicateProvider",
            duplicate_type_id,
            &[],
            ProviderVisibility::Public,
            SourceLocation::new("duplicate_reachable.rs", 1, 1),
            duplicate_constructor,
        )
        .with_namespace(module_path!())
    }

    mads_core::__private::inventory::submit! {
        ProviderDescriptor::new(
            ProviderKind::Provider,
            "DuplicateProvider",
            duplicate_type_id,
            &[],
            ProviderVisibility::Public,
            SourceLocation::new("duplicate_reachable.rs", 1, 1),
            duplicate_constructor,
        )
        .with_namespace(module_path!())
    }
}

mod ambiguous_dependency {
    #[derive(Clone)]
    pub struct SharedDependency;

    pub mod app {
        use super::SharedDependency;

        #[mads_core::furnace]
        pub struct AmbiguousDependencyRoot;

        impl mads_core::Furnace for AmbiguousDependencyRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<SharedDependency>()
                    .provide::<Consumer>()
                    .export::<Consumer>()
            }
        }

        pub struct Consumer;

        #[mads_core::element]
        pub fn consumer(_dependency: SharedDependency) -> Consumer {
            Consumer
        }
    }

    pub mod constructors {
        use super::SharedDependency;

        #[mads_core::element]
        pub fn first_shared_dependency() -> SharedDependency {
            SharedDependency
        }

        #[mads_core::element]
        pub fn second_shared_dependency() -> SharedDependency {
            SharedDependency
        }
    }
}

mod mixed_dependency_scope {
    #[derive(Clone)]
    pub struct SharedDependency;

    pub mod allowed {
        use super::SharedDependency;

        #[mads_core::furnace]
        pub struct AllowedModule;

        impl mads_core::Furnace for AllowedModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<SharedDependency>()
                    .export::<SharedDependency>()
            }
        }

        #[mads_core::element]
        pub fn allowed_dependency() -> SharedDependency {
            SharedDependency
        }
    }

    pub mod foreign {
        use super::SharedDependency;

        #[mads_core::furnace]
        pub struct ForeignModule;

        impl mads_core::Furnace for ForeignModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<SharedDependency>()
                    .export::<SharedDependency>()
            }
        }

        #[mads_core::element]
        pub fn foreign_dependency() -> SharedDependency {
            SharedDependency
        }
    }

    pub mod app {
        use super::{SharedDependency, allowed::AllowedModule};

        #[mads_core::furnace]
        pub struct MixedDependencyRoot;

        impl mads_core::Furnace for MixedDependencyRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<Consumer>()
                    .import(AllowedModule)
                    .export::<Consumer>()
            }
        }

        pub struct Consumer;

        #[mads_core::element]
        pub fn consumer(_dependency: SharedDependency) -> Consumer {
            Consumer
        }
    }
}

fn rooted_analysis<M: Furnace>() -> GraphAnalysis {
    let mut builder = Mads::builder();
    builder
        .root::<M>()
        .expect("module graph metadata should be valid");
    builder.analyze()
}

#[test]
fn rooted_scope_includes_only_explicit_members_and_reachable_furnaces() {
    use selected_scope::{
        app::{AppModule, ReachableController, UnusedButOwnedService},
        unowned::RequiredUnownedUseCase,
        unreachable::UnreachableService,
    };

    let analysis = rooted_analysis::<AppModule>();
    assert!(analysis.is_valid());
    assert!(analysis.graph().provider::<ReachableController>().is_some());
    assert!(
        analysis
            .graph()
            .provider::<UnusedButOwnedService>()
            .is_some()
    );
    assert!(analysis.graph().provider::<UnreachableService>().is_none());
    assert!(
        analysis
            .graph()
            .provider::<RequiredUnownedUseCase>()
            .is_some()
    );
}

#[test]
fn rooted_scope_reports_duplicate_reachable_provider_declarations() {
    let analysis = rooted_analysis::<duplicate_reachable::DuplicateRoot>();

    assert_eq!(analysis.diagnostics()[0].code(), MADS001);
    assert!(analysis.construction_plan().is_none());
}

#[test]
fn rooted_scope_reports_ambiguous_registered_output_constructors() {
    let analysis = rooted_analysis::<ambiguous_dependency::app::AmbiguousDependencyRoot>();

    assert_eq!(analysis.diagnostics()[0].code(), MADS002);
    assert!(analysis.construction_plan().is_none());
}

#[test]
fn externally_satisfied_dependency_skips_ambiguous_static_constructors() {
    let mut builder = Mads::builder();
    builder
        .root::<ambiguous_dependency::app::AmbiguousDependencyRoot>()
        .unwrap()
        .provide(ambiguous_dependency::SharedDependency)
        .unwrap();

    let analysis = builder.analyze();
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    assert!(
        analysis
            .graph()
            .provider::<ambiguous_dependency::SharedDependency>()
            .is_some()
    );
}

#[test]
fn registering_output_with_multiple_catalog_factories_is_ambiguous() {
    let analysis = rooted_analysis::<mixed_dependency_scope::app::MixedDependencyRoot>();
    assert_eq!(analysis.diagnostics()[0].code(), MADS002);
    assert!(analysis.construction_plan().is_none());
}

#[test]
fn direct_import_allows_an_unrestricted_public_provider() {
    let analysis = rooted_analysis::<direct_public::root::DirectRoot>();
    assert!(analysis.is_valid());
}

#[test]
fn global_module_exposes_public_providers_without_a_direct_import() {
    use global_public::{app::GlobalRoot, database::DatabasePool, user::UserService};

    let analysis = rooted_analysis::<GlobalRoot>();

    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    assert!(analysis.graph().provider::<DatabasePool>().is_some());
    assert!(analysis.graph().provider::<UserService>().is_some());
}

#[test]
fn unreachable_provider_does_not_satisfy_a_dependency() {
    let analysis = rooted_analysis::<missing_import::root::MissingImportRoot>();
    assert_eq!(analysis.diagnostics()[0].code(), mads_core::MADS003);
    assert!(analysis.construction_plan().is_none());
}

#[test]
fn transitive_import_does_not_reexport_a_provider() {
    let analysis = rooted_analysis::<transitive_only::root::TransitiveRoot>();
    assert_eq!(analysis.diagnostics()[0].code(), MADS009);
    assert!(
        analysis.diagnostics()[0]
            .to_string()
            .contains("direct import")
    );
}

#[test]
fn restricted_provider_cannot_cross_a_module_boundary() {
    let analysis = rooted_analysis::<restricted_crossing::root::RestrictedRoot>();
    assert_eq!(analysis.diagnostics()[0].code(), MADS009);
    assert!(
        analysis.diagnostics()[0]
            .to_string()
            .contains("explicit export")
    );
}

#[test]
fn linked_unregistered_bridge_is_not_implicitly_selected() {
    let analysis = rooted_analysis::<unowned_bridge::root::BridgeRoot>();
    assert_eq!(analysis.diagnostics()[0].code(), mads_core::MADS003);
}

#[tokio::test]
async fn diamond_scope_plans_and_constructs_the_shared_provider_once() {
    use diamond::{CONSTRUCTIONS, root::DiamondRoot, shared::SharedProvider};

    CONSTRUCTIONS.store(0, Ordering::SeqCst);
    let analysis = rooted_analysis::<DiamondRoot>();
    let plan = analysis
        .construction_plan()
        .expect("diamond scope should have a construction plan");
    assert_eq!(
        plan.steps()
            .iter()
            .filter(|step| step.type_name() == "SharedProvider")
            .count(),
        1
    );

    let registry = ProviderRegistry::new();
    let config = Config::empty();
    let context = ConstructionContext::new(&registry, &config);
    for step in plan.steps() {
        let _ = (step.__descriptor().constructor())(&context)
            .await
            .expect("planned constructor should succeed");
    }
    assert!(analysis.graph().provider::<SharedProvider>().is_some());
    assert_eq!(CONSTRUCTIONS.load(Ordering::SeqCst), 1);
}
