//! Activation and provisioning behavior for the official JWT default.

#![cfg(feature = "jwt")]

use std::time::Duration;

use furnace_rs_common::{FURNACE121, JwtService, JwtSignOptions, JwtValidation};
use furnace_rs_core::{
    AutoConfigurationStatus, Config, ConfigBuilder, FURNACE003, Furnace, MapSource, ProviderOrigin,
};

#[furnace_rs_core::burner]
struct TokenIssuer {
    jwt: JwtService,
}

fn config(values: impl IntoIterator<Item = (&'static str, &'static str)>) -> Config {
    ConfigBuilder::new()
        .source(MapSource::new("furnace.toml", values))
        .build()
        .unwrap()
}

fn jwt_report(
    analysis: &furnace_rs_core::GraphAnalysis,
) -> &furnace_rs_core::AutoConfigurationReport {
    analysis
        .auto_configurations()
        .iter()
        .find(|report| report.identifier() == "furnace.common.passport.jwt")
        .expect("the JWT auto-configuration descriptor must be registered")
}

#[test]
fn direct_requirement_activates_the_jwt_default() {
    let analysis = Furnace::builder_with_config(config([(
        "passport.secret",
        "01234567890123456789012345678901",
    )]))
    .analyze();
    let report = jwt_report(&analysis);

    assert!(analysis.is_valid());
    assert_eq!(report.status(), AutoConfigurationStatus::Active);
    assert_eq!(report.reason_code().as_str(), "conditions_matched");
    assert!(
        report.requirements()[0]
            .provider_type_name()
            .contains("TokenIssuer")
    );
    assert_eq!(report.configuration()[0].key(), "passport.secret");
    assert_eq!(report.configuration()[0].source(), Some("furnace.toml"));
    assert!(!format!("{report:?}").contains("01234567890123456789012345678901"));
}

#[test]
fn missing_required_configuration_is_furnace121_without_furnace003() {
    let analysis = Furnace::builder().analyze();
    let report = jwt_report(&analysis);

    assert!(!analysis.is_valid());
    assert_eq!(report.status(), AutoConfigurationStatus::Failed);
    assert_eq!(report.reason_code().as_str(), "missing_configuration");
    assert_eq!(analysis.diagnostics()[0].code(), FURNACE121);
    assert!(
        analysis
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code() != FURNACE003)
    );
    let rendered = analysis.diagnostics()[0].to_string();
    assert!(rendered.contains("passport.secret"));
    assert!(rendered.contains("TokenIssuer"));
    assert!(rendered.contains("explicit `JwtService`"));
}

#[test]
fn invalid_required_configuration_is_furnace121_and_redacted() {
    const SENTINEL: &str = "jwt-secret-never-display";
    let analysis = Furnace::builder_with_config(config([("passport.secret", SENTINEL)])).analyze();
    let report = jwt_report(&analysis);

    assert!(!analysis.is_valid());
    assert_eq!(report.status(), AutoConfigurationStatus::Failed);
    assert_eq!(report.reason_code().as_str(), "invalid_configuration");
    assert_eq!(analysis.diagnostics()[0].code(), FURNACE121);
    assert!(
        analysis
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code() != FURNACE003)
    );
    assert!(!format!("{:?}", analysis.auto_configurations()).contains(SENTINEL));
    assert!(!analysis.diagnostics()[0].to_string().contains(SENTINEL));
}

#[tokio::test]
async fn build_injects_the_cloneable_auto_configured_service_into_its_consumer() {
    let application = Furnace::builder_with_config(config([(
        "passport.secret",
        "01234567890123456789012345678901",
    )]))
    .build()
    .await
    .unwrap();
    let issuer = application.context().resolve::<TokenIssuer>().unwrap();
    let service = application.context().resolve::<JwtService>().unwrap();
    let token = issuer
        .jwt
        .clone()
        .sign(
            serde_json::json!({ "user_id": 7 }),
            JwtSignOptions::access(Duration::from_secs(60)),
        )
        .unwrap();

    assert!(
        service
            .verify::<serde_json::Value>(&token, JwtValidation::access())
            .is_ok()
    );
    assert_eq!(
        application
            .graph()
            .provider::<JwtService>()
            .unwrap()
            .origin(),
        ProviderOrigin::AutoConfiguration,
    );
}
