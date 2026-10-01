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

#[furnace_rs::element]
fn configured_value(config: Config) -> ConfiguredValue {
    ConfiguredValue(
        config
            .get("application.name")
            .expect("the test configuration contains an application name")
            .to_owned(),
    )
}

#[furnace_rs::element]
async fn combined_value(
    config: Config,
    configured: ConfiguredValue,
) -> furnace_rs::core::Result<CombinedValue> {
    Ok(CombinedValue {
        configured: configured.0,
        entries: config.len(),
    })
}

#[furnace_rs::element]
/// Public provider used to verify visibility metadata.
pub fn public_provider() -> PublicProviderValue {
    PublicProviderValue
}

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

    assert_eq!(dependency_names, ["Config", "ConfiguredValue"]);
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
