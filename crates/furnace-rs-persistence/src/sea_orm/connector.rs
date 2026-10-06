use std::fmt;

use ::sea_orm::{ConnectOptions, DatabaseConnection, SqlxPostgresConnector};
use furnace_rs_core::Config;
use tracing::instrument::WithSubscriber;

use crate::{DatabaseConnector, PersistenceError, PersistenceErrorKind, PersistenceResult};

use super::config::{SeaOrmConfig, deadline_is_representable};

/// A connector that returns SeaORM's native PostgreSQL database connection.
///
/// Native connection-establishment tracing is suppressed because SeaORM records
/// credential-bearing options in its connection span. Application tracing and
/// statement logging on the returned connection retain their configured behavior.
#[derive(Clone)]
pub struct SeaOrmPostgres {
    options: ConnectOptions,
}

impl SeaOrmPostgres {
    /// Creates a connector from a PostgreSQL URL without opening a connection.
    pub fn new(url: impl Into<String>) -> Self {
        Self::from_options(ConnectOptions::new(url))
    }

    /// Creates a connector from caller-configured native SeaORM options.
    pub fn from_options(options: ConnectOptions) -> Self {
        Self { options }
    }

    /// Returns mutable access to native SeaORM connection options.
    pub fn options_mut(&mut self) -> &mut ConnectOptions {
        &mut self.options
    }

    #[allow(dead_code)] // Used by DatabaseCauldron in the next implementation task.
    pub(crate) fn from_config(config: &Config) -> PersistenceResult<Self> {
        let config = SeaOrmConfig::parse(config).map_err(|source| {
            PersistenceError::with_source(
                PersistenceErrorKind::InvalidConfiguration,
                "configure",
                source,
            )
        })?;
        let mut connector = Self::new(config.url.expose().clone());
        config.apply(connector.options_mut());
        Ok(connector)
    }
}

impl fmt::Debug for SeaOrmPostgres {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SeaOrmPostgres")
            .field("backend", &"postgres")
            .finish()
    }
}

impl DatabaseConnector for SeaOrmPostgres {
    type Database = DatabaseConnection;

    async fn connect(self) -> PersistenceResult<Self::Database> {
        validate_postgres_scheme(self.options.get_url())?;
        // A zero reaping interval continuously reschedules SQLx maintenance.
        // Native callers can disable these policies with None instead.
        if [
            self.options.get_idle_timeout().flatten(),
            self.options.get_max_lifetime().flatten(),
        ]
        .into_iter()
        .flatten()
        .any(|duration| duration.is_zero())
        {
            return Err(PersistenceError::new(
                PersistenceErrorKind::InvalidConfiguration,
                "configure",
            ));
        }
        // SQLx adds pool timeouts to an Instant without checking overflow.
        // Validate native options too, including options changed after parsing.
        for duration in [
            self.options.get_connect_timeout(),
            self.options.get_acquire_timeout(),
            self.options.get_idle_timeout().flatten(),
            self.options.get_max_lifetime().flatten(),
        ]
        .into_iter()
        .flatten()
        {
            if !deadline_is_representable(duration) {
                return Err(PersistenceError::new(
                    PersistenceErrorKind::InvalidConfiguration,
                    "configure",
                ));
            }
        }
        SqlxPostgresConnector::connect(self.options)
            // SeaORM's connection span records the full URL in ConnectOptions.
            // Scope the dispatcher to each poll of this future, never globally.
            .with_subscriber(tracing::subscriber::NoSubscriber::default())
            .await
            .map_err(|source| {
                PersistenceError::with_source(PersistenceErrorKind::Connection, "connect", source)
            })
    }
}

fn validate_postgres_scheme(url: &str) -> PersistenceResult<()> {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        Ok(())
    } else {
        Err(PersistenceError::new(
            PersistenceErrorKind::UnsupportedScheme,
            "connect",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_matrix() {
        for accepted in ["postgres://localhost/db", "postgresql://localhost/db"] {
            assert!(validate_postgres_scheme(accepted).is_ok());
        }
        for rejected in [
            "mysql://localhost/db",
            "sqlite://db.sqlite",
            "localhost/db",
            "",
        ] {
            assert_eq!(
                validate_postgres_scheme(rejected).unwrap_err().kind(),
                PersistenceErrorKind::UnsupportedScheme
            );
        }
    }
}
