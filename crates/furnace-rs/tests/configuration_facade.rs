//! Feature-independent typed configuration and provider construction boundaries.

use furnace_rs::prelude::*;
use furnace_rs::{ConfigurationErrors, ConfigurationIssue, ConfigurationResult, Secret};
use std::sync::atomic::{AtomicUsize, Ordering};

struct StartCounter(std::sync::Arc<AtomicUsize>);

impl LifecycleHook for StartCounter {
    fn name(&self) -> &str {
        "configuration-start-boundary"
    }
    fn start<'a>(&'a self, _: &'a ApplicationContext) -> furnace_rs::core::LifecycleFuture<'a> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
    fn stop<'a>(&'a self, _: &'a ApplicationContext) -> furnace_rs::core::LifecycleFuture<'a> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(furnace_rs::Configuration, Debug, Clone)]
#[config(prefix = "app")]
struct AppConfig {
    api_key: Secret<String>,
}

#[test]
fn facade_and_prelude_expose_the_same_configuration_contract() {
    fn accepts_result(_: ConfigurationResult<()>) {}
    let issue = ConfigurationIssue::new(
        "app.api_key",
        "required",
        "required configuration key is missing",
    );
    accepts_result(Err(ConfigurationErrors::from_issue(issue)));
    let errors = Config::empty().parse::<AppConfig>().unwrap_err();
    assert_eq!(errors.issues()[0].key(), "app.api_key");
}

mod selected {
    use super::*;

    pub static CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);

    #[cauldron]
    pub struct AppCauldron;

    impl furnace_rs::core::Cauldron for AppCauldron {
        fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
            self.provide_with::<AppConfig, AppConfigInjector>()
                .provide_with::<Consumer, ConsumerInjector>()
        }
    }

    fn app_config(config: Config) -> furnace_rs::core::Result<AppConfig> {
        Ok(config.parse()?)
    }
    #[doc = "Explicit constructor for the fixture's provider output."]
    struct AppConfigInjector;
    impl furnace_rs_core::Injector<AppConfig> for AppConfigInjector {
        type Dependencies = (Config,);
        async fn inject((dependency_0,): Self::Dependencies) -> furnace_rs_core::Result<AppConfig> {
            app_config(dependency_0)
        }
        fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
            &__FURNACE_INJECTOR_DESCRIPTOR_APP_CONFIG
        }
    }
    const __FURNACE_INJECTOR_DESCRIPTOR_APP_CONFIG: furnace_rs_core::ProviderDescriptor =
        furnace_rs_core::__private::InjectorMetadata::<AppConfig, AppConfigInjector>::DESCRIPTOR
            .with_authored_type_name("AppConfig")
            .with_namespace(module_path!())
            .with_visibility(furnace_rs_core::ProviderVisibility::Private)
            .with_location(furnace_rs_core::SourceLocation::new(
                file!(),
                line!(),
                column!(),
            ));
    furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_APP_CONFIG }

    pub struct Consumer;

    fn consumer(config: AppConfig) -> Consumer {
        let _ = config.api_key.expose();
        CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
        Consumer
    }
    #[doc = "Explicit constructor for the fixture's provider output."]
    struct ConsumerInjector;
    impl furnace_rs_core::Injector<Consumer> for ConsumerInjector {
        type Dependencies = (AppConfig,);
        async fn inject((dependency_0,): Self::Dependencies) -> furnace_rs_core::Result<Consumer> {
            Ok(consumer(dependency_0))
        }
        fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
            &__FURNACE_INJECTOR_DESCRIPTOR_CONSUMER
        }
    }
    const __FURNACE_INJECTOR_DESCRIPTOR_CONSUMER: furnace_rs_core::ProviderDescriptor =
        furnace_rs_core::__private::InjectorMetadata::<Consumer, ConsumerInjector>::DESCRIPTOR
            .with_authored_type_name("Consumer")
            .with_namespace(module_path!())
            .with_visibility(furnace_rs_core::ProviderVisibility::Private)
            .with_location(furnace_rs_core::SourceLocation::new(
                file!(),
                line!(),
                column!(),
            ));
    furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_CONSUMER }
}

#[tokio::test]
async fn selected_provider_failure_prevents_dependent_construction() {
    let mut builder = Furnace::builder_with_config(Config::empty());
    let starts = std::sync::Arc::new(AtomicUsize::new(0));
    builder.lifecycle_hook(StartCounter(starts.clone()));
    builder.root::<selected::AppCauldron>().unwrap();
    let error = builder
        .build()
        .await
        .err()
        .expect("missing configuration must fail construction");
    assert_eq!(error.code(), furnace_rs::core::FURNACE006);
    let cause = std::error::Error::source(&error)
        .and_then(|source| source.downcast_ref::<furnace_rs::core::Error>())
        .expect("construction failure retains the structured configuration cause");
    assert_eq!(cause.code(), furnace_rs::core::FURNACE020);
    assert_eq!(cause.diagnostic().subject(), Some("app.api_key"));
    assert_eq!(selected::CONSTRUCTIONS.load(Ordering::SeqCst), 0);
    assert_eq!(starts.load(Ordering::SeqCst), 0);
}

mod unused {
    use super::*;

    #[cauldron]
    pub struct EmptyCauldron;

    impl furnace_rs::core::Cauldron for EmptyCauldron {
        fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
            furnace_rs::core::CauldronRegistration::new(self)
        }
    }

    #[derive(Configuration)]
    #[allow(dead_code)]
    struct Unused {
        required_secret: Secret<String>,
    }
}

#[tokio::test]
async fn unused_derive_does_not_register_a_startup_requirement() {
    let mut builder = Furnace::builder_with_config(Config::empty());
    builder.root::<unused::EmptyCauldron>().unwrap();
    assert!(builder.build().await.is_ok());
}
