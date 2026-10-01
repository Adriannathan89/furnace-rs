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
            self.provide::<AppConfig>().provide::<Consumer>()
        }
    }

    #[element]
    fn app_config(config: Config) -> furnace_rs::core::Result<AppConfig> {
        Ok(config.parse()?)
    }

    pub struct Consumer;

    #[element]
    fn consumer(config: AppConfig) -> Consumer {
        let _ = config.api_key.expose();
        CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
        Consumer
    }
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
