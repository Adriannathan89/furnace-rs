//! Integration tests for explicit provider-function construction.

use furnace_rs::core::{
    Catalog, Config, ConfigBuilder, FURNACE003, Furnace, MapSource, ProviderVisibility,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ConfiguredValue(String);

#[derive(Clone, Debug, Eq, PartialEq)]
struct CombinedValue {
    configured: String,
    entries: usize,
}

/// Output type for the public provider visibility fixture.
pub struct PublicProviderValue;

fn configured_value(config: Config) -> ConfiguredValue {
    ConfiguredValue(
        config
            .get("application.name")
            .expect("the test configuration contains an application name")
            .to_owned(),
    )
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct ConfiguredValueInjector;
impl furnace_rs::core::Injector<ConfiguredValue> for ConfiguredValueInjector {
    type Dependencies = (Config,);
    async fn inject(
        (dependency_0,): Self::Dependencies,
    ) -> furnace_rs::core::Result<ConfiguredValue> {
        Ok(configured_value(dependency_0))
    }
    fn descriptor() -> &'static furnace_rs::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_CONFIGURED_VALUE
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_CONFIGURED_VALUE : furnace_rs :: core :: ProviderDescriptor = furnace_rs :: core :: __private :: InjectorMetadata :: < ConfiguredValue , ConfiguredValueInjector > :: DESCRIPTOR . with_authored_type_name (stringify ! (ConfiguredValue)) . with_namespace (module_path ! ()) . with_visibility (furnace_rs :: core :: ProviderVisibility :: Private) . with_location (furnace_rs :: core :: SourceLocation :: new (file ! () , line ! () , column ! ())) ;
furnace_rs::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_CONFIGURED_VALUE }

async fn combined_value(
    config: Config,
    configured: ConfiguredValue,
) -> furnace_rs::core::Result<CombinedValue> {
    Ok(CombinedValue {
        configured: configured.0,
        entries: config.len(),
    })
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct CombinedValueInjector;
impl furnace_rs::core::Injector<CombinedValue> for CombinedValueInjector {
    type Dependencies = (Config, ConfiguredValue);
    async fn inject(
        (dependency_0, dependency_1): Self::Dependencies,
    ) -> furnace_rs::core::Result<CombinedValue> {
        combined_value(dependency_0, dependency_1).await
    }
    fn descriptor() -> &'static furnace_rs::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_COMBINED_VALUE
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_COMBINED_VALUE : furnace_rs :: core :: ProviderDescriptor = furnace_rs :: core :: __private :: InjectorMetadata :: < CombinedValue , CombinedValueInjector > :: DESCRIPTOR . with_authored_type_name (stringify ! (CombinedValue)) . with_namespace (module_path ! ()) . with_visibility (furnace_rs :: core :: ProviderVisibility :: Private) . with_location (furnace_rs :: core :: SourceLocation :: new (file ! () , line ! () , column ! ())) ;
furnace_rs::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_COMBINED_VALUE }

/// Public provider used to verify visibility metadata.
pub fn public_provider() -> PublicProviderValue {
    PublicProviderValue
}
#[doc = "Explicit constructor for the fixture's provider output."]
pub struct PublicProviderInjector;
impl furnace_rs::core::Injector<PublicProviderValue> for PublicProviderInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs::core::Result<PublicProviderValue> {
        Ok(public_provider())
    }
    fn descriptor() -> &'static furnace_rs::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_PUBLIC_PROVIDER
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_PUBLIC_PROVIDER : furnace_rs :: core :: ProviderDescriptor = furnace_rs :: core :: __private :: InjectorMetadata :: < PublicProviderValue , PublicProviderInjector > :: DESCRIPTOR . with_authored_type_name (stringify ! (PublicProviderValue)) . with_namespace (module_path ! ()) . with_visibility (furnace_rs :: core :: ProviderVisibility :: Public) . with_location (furnace_rs :: core :: SourceLocation :: new (file ! () , line ! () , column ! ())) ;
furnace_rs::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_PUBLIC_PROVIDER }

fn test_config() -> Config {
    ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [("application.name", "provider-test")],
        ))
        .build()
        .expect("the fixed test configuration should build")
}

#[test]
fn provider_dependencies_follow_parameter_order() {
    let descriptor = Catalog::provider_for::<CombinedValue>()
        .expect("the combined-value provider descriptor should be registered");
    let dependency_names: Vec<_> = descriptor
        .dependencies()
        .iter()
        .map(|dependency| dependency.type_name())
        .collect();

    assert_eq!(
        dependency_names,
        [
            "furnace_rs_core::config::Config",
            "provider::ConfiguredValue"
        ]
    );
}

#[test]
fn provider_visibility_matches_function_visibility() {
    assert_eq!(
        Catalog::provider_for::<ConfiguredValue>()
            .expect("private provider descriptor should be registered")
            .visibility(),
        ProviderVisibility::Private,
    );
    assert_eq!(
        Catalog::provider_for::<CombinedValue>()
            .expect("private async provider descriptor should be registered")
            .visibility(),
        ProviderVisibility::Private,
    );
    assert_eq!(
        Catalog::provider_for::<PublicProviderValue>()
            .expect("public provider descriptor should be registered")
            .visibility(),
        ProviderVisibility::Public,
    );
}

#[tokio::test]
async fn automatic_build_stores_direct_and_fallible_provider_outputs() {
    let application = Furnace::builder_with_config(test_config())
        .build()
        .await
        .expect("the provider graph should build");

    assert!(application.context().resolve::<ConfiguredValue>().is_ok());
    assert!(application.context().resolve::<CombinedValue>().is_ok());
}

#[tokio::test]
async fn construct_does_not_recursively_build_a_missing_provider_dependency() {
    let mut builder = Furnace::builder_with_config(test_config());

    let Err(error) = builder.construct::<CombinedValue>().await else {
        panic!("combined construction should require explicit configured-value construction");
    };

    assert_eq!(error.code(), FURNACE003);
}
