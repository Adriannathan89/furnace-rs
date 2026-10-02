use std::time::Duration;

use ::sea_orm::ConnectOptions;
use furnace_rs_core::{Config, Configuration, ConfigurationErrors, ConfigurationIssue, Secret};

#[derive(Configuration, Debug)]
#[config(prefix = "")]
pub(super) struct SeaOrmConfig {
    pub(super) url: Secret<String>,
    #[config(validate(positive))]
    min_connections: Option<u32>,
    #[config(validate(positive))]
    max_connections: Option<u32>,
    connect_timeout_seconds: Option<u64>,
    acquire_timeout_seconds: Option<u64>,
    idle_timeout_seconds: Option<u64>,
    max_lifetime_seconds: Option<u64>,
    sqlx_logging: Option<bool>,
}

#[derive(Configuration)]
#[config(prefix = "persistence")]
struct PersistenceConfig {
    seaorm: SeaOrmConfig,
}

impl SeaOrmConfig {
    pub(super) fn parse(config: &Config) -> Result<Self, ConfigurationErrors> {
        let parsed = config.parse::<PersistenceConfig>()?.seaorm;
        for (field, seconds) in [
            ("connect_timeout_seconds", parsed.connect_timeout_seconds),
            ("acquire_timeout_seconds", parsed.acquire_timeout_seconds),
            ("idle_timeout_seconds", parsed.idle_timeout_seconds),
            ("max_lifetime_seconds", parsed.max_lifetime_seconds),
        ] {
            if let Some(seconds) = seconds
                && !deadline_is_representable(Duration::from_secs(seconds))
            {
                let key = format!("persistence.seaorm.{field}");
                let mut issue = ConfigurationIssue::new(
                    &key,
                    "out_of_range",
                    "duration exceeds the supported deadline range",
                );
                if let Some(source) = config.source_of(&key) {
                    issue = issue.with_source(source);
                }
                return Err(ConfigurationErrors::from_issue(issue));
            }
        }
        if let (Some(min), Some(max)) = (parsed.min_connections, parsed.max_connections)
            && min > max
        {
            let mut issue = ConfigurationIssue::new(
                "persistence.seaorm.min_connections",
                "invalid_relationship",
                "minimum connections must not exceed maximum connections",
            );
            if let Some(source) = config.source_of("persistence.seaorm.min_connections") {
                issue = issue.with_source(source);
            }
            return Err(ConfigurationErrors::from_issue(issue));
        }
        Ok(parsed)
    }

    pub(super) fn apply(&self, options: &mut ConnectOptions) {
        if let Some(value) = self.min_connections {
            options.min_connections(value);
        }
        if let Some(value) = self.max_connections {
            options.max_connections(value);
        }
        if let Some(value) = self.connect_timeout_seconds {
            options.connect_timeout(Duration::from_secs(value));
        }
        if let Some(value) = self.acquire_timeout_seconds {
            options.acquire_timeout(Duration::from_secs(value));
        }
        if let Some(value) = self.idle_timeout_seconds {
            options.idle_timeout(Duration::from_secs(value));
        }
        if let Some(value) = self.max_lifetime_seconds {
            options.max_lifetime(Duration::from_secs(value));
        }
        if let Some(value) = self.sqlx_logging {
            options.sqlx_logging(value);
        }
    }
}

pub(super) fn deadline_is_representable(duration: Duration) -> bool {
    std::time::Instant::now().checked_add(duration).is_some()
}

#[cfg(test)]
mod tests {
    use furnace_rs_core::{ConfigBuilder, MapSource};

    use super::*;

    fn config(values: impl IntoIterator<Item = (&'static str, &'static str)>) -> Config {
        ConfigBuilder::new()
            .source(MapSource::new("fixture", values))
            .build()
            .unwrap()
    }

    #[test]
    fn required_url_and_optional_defaults() {
        let missing = SeaOrmConfig::parse(&Config::empty()).unwrap_err();
        assert_eq!(missing.issues()[0].key(), "persistence.seaorm.url");
        let settings = SeaOrmConfig::parse(&config([(
            "persistence.seaorm.url",
            "postgres://localhost/db",
        )]))
        .unwrap();
        let mut options = ConnectOptions::new("postgres://localhost/db");
        settings.apply(&mut options);
        assert_eq!(options.get_min_connections(), None);
        assert_eq!(options.get_max_connections(), None);
        assert_eq!(options.get_connect_timeout(), None);
        assert_eq!(options.get_acquire_timeout(), None);
        assert_eq!(options.get_idle_timeout(), None);
        assert_eq!(options.get_max_lifetime(), None);
    }

    #[test]
    fn explicit_values_and_relationship_validation() {
        let values = [
            ("persistence.seaorm.url", "postgres://localhost/db"),
            ("persistence.seaorm.min_connections", "2"),
            ("persistence.seaorm.max_connections", "5"),
            ("persistence.seaorm.connect_timeout_seconds", "3"),
            ("persistence.seaorm.acquire_timeout_seconds", "4"),
            ("persistence.seaorm.idle_timeout_seconds", "5"),
            ("persistence.seaorm.max_lifetime_seconds", "6"),
            ("persistence.seaorm.sqlx_logging", "false"),
        ];
        let settings = SeaOrmConfig::parse(&config(values)).unwrap();
        let mut options = ConnectOptions::new("postgres://localhost/db");
        settings.apply(&mut options);
        assert_eq!(options.get_min_connections(), Some(2));
        assert_eq!(options.get_max_connections(), Some(5));
        assert_eq!(options.get_connect_timeout(), Some(Duration::from_secs(3)));
        assert_eq!(options.get_acquire_timeout(), Some(Duration::from_secs(4)));
        assert_eq!(
            options.get_idle_timeout(),
            Some(Some(Duration::from_secs(5)))
        );
        assert_eq!(
            options.get_max_lifetime(),
            Some(Some(Duration::from_secs(6)))
        );
        assert!(!options.get_sqlx_logging());
        let bad = config([
            values[0],
            ("persistence.seaorm.min_connections", "6"),
            values[2],
        ]);
        let errors = SeaOrmConfig::parse(&bad).unwrap_err();
        let issue = &errors.issues()[0];
        assert_eq!(issue.key(), "persistence.seaorm.min_connections");
        assert_eq!(issue.code(), "invalid_relationship");
        assert_eq!(
            issue.message(),
            "minimum connections must not exceed maximum connections"
        );
        assert_eq!(issue.source(), Some("fixture"));
    }

    #[test]
    fn zero_counts_rejected_on_own_keys() {
        for key in [
            "persistence.seaorm.min_connections",
            "persistence.seaorm.max_connections",
        ] {
            let errors = SeaOrmConfig::parse(&config([
                ("persistence.seaorm.url", "postgres://localhost/db"),
                (key, "0"),
            ]))
            .unwrap_err();
            assert_eq!(errors.issues()[0].key(), key);
            assert_eq!(errors.issues()[0].code(), "too_small");
        }
    }

    #[test]
    fn overflowing_timeout_seconds_are_rejected_with_safe_key_and_source() {
        for key in [
            "persistence.seaorm.connect_timeout_seconds",
            "persistence.seaorm.acquire_timeout_seconds",
            "persistence.seaorm.idle_timeout_seconds",
            "persistence.seaorm.max_lifetime_seconds",
        ] {
            let settings = config([
                (
                    "persistence.seaorm.url",
                    "postgres://user:timeout-secret-password@localhost/db",
                ),
                (key, "18446744073709551615"),
            ]);
            let errors = SeaOrmConfig::parse(&settings).unwrap_err();
            assert_eq!(errors.issues()[0].key(), key);
            assert_eq!(errors.issues()[0].source(), Some("fixture"));
            assert_eq!(errors.issues()[0].code(), "out_of_range");
            assert!(!format!("{errors:?} {errors}").contains("timeout-secret-password"));
            assert!(!format!("{errors:?} {errors}").contains("18446744073709551615"));
        }
    }

    #[test]
    fn secret_url_stays_out_of_config_and_error_formatting() {
        use crate::PersistenceError;
        let secret = "postgres://furnace_rs-secret-user:furnace_rs-secret-password@localhost/furnace_rs?token=furnace_rs-secret-query";
        let config = ConfigBuilder::new()
            .source(MapSource::new(
                "fixture",
                [
                    ("persistence.seaorm.url", secret),
                    ("persistence.seaorm.min_connections", "4"),
                    ("persistence.seaorm.max_connections", "2"),
                ],
            ))
            .build()
            .unwrap();
        assert!(!format!("{config:?}").contains("furnace_rs-secret"));
        let error = super::super::SeaOrmPostgres::from_config(&config).unwrap_err();
        assert!(!format!("{error:?} {error}").contains("furnace_rs-secret"));
        let framework: furnace_rs_core::Error = error.into();
        assert!(!format!("{framework:?} {framework}").contains("furnace_rs-secret"));
        assert!(
            std::error::Error::source(&framework)
                .and_then(|source| source.downcast_ref::<PersistenceError>())
                .is_some()
        );
    }
}
