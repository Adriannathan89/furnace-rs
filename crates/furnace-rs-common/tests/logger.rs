//! Public API coverage for the optional logger integration.

#![cfg(feature = "logger")]

use std::sync::{Arc, Mutex};

use furnace_rs_common::{LogLevel, Logger, LoggerService};
use furnace_rs_core::Furnace;

#[derive(Clone, Default)]
struct RecordingLogger {
    entries: Arc<Mutex<Vec<(LogLevel, String)>>>,
}

impl LoggerService for RecordingLogger {
    fn log(&self, level: LogLevel, message: &str) {
        self.entries
            .lock()
            .unwrap()
            .push((level, message.to_owned()));
    }
}

#[test]
fn logger_delegates_messages_to_the_configured_inner_logger() {
    let inner = RecordingLogger::default();
    let entries = Arc::clone(&inner.entries);
    let logger = Logger::new(inner);

    logger.error("database is unavailable");

    let entries = entries.lock().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(matches!(entries[0].0, LogLevel::Error));
    assert_eq!(entries[0].1, "database is unavailable");
}

#[test]
fn logger_delegates_info_messages_to_the_configured_inner_logger() {
    let inner = RecordingLogger::default();
    let entries = Arc::clone(&inner.entries);
    let logger = Logger::new(inner);

    logger.info("server started");

    let entries = entries.lock().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(matches!(entries[0].0, LogLevel::Info));
    assert_eq!(entries[0].1, "server started");
}

#[test]
fn logger_includes_trace_and_context_details_in_trace_messages() {
    let inner = RecordingLogger::default();
    let entries = Arc::clone(&inner.entries);
    let logger = Logger::new(inner);

    logger.trace(
        "request failed",
        Some("database::connect"),
        Some("request_id=req-42"),
    );

    let entries = entries.lock().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(matches!(entries[0].0, LogLevel::Trace));
    assert_eq!(
        entries[0].1,
        "request failed\nTrace: database::connect\nContext: request_id=req-42"
    );
}

#[test]
fn logger_includes_context_details_in_fatal_messages() {
    let inner = RecordingLogger::default();
    let entries = Arc::clone(&inner.entries);
    let logger = Logger::new(inner);

    logger.fatal("application cannot continue", Some("migration=20260922"));

    let entries = entries.lock().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(matches!(entries[0].0, LogLevel::Fatal));
    assert_eq!(
        entries[0].1,
        "application cannot continue\nContext: migration=20260922"
    );
}

mod consumer {
    use furnace_rs_common::Logger;

    #[furnace_rs_core::burner]
    pub struct LoggingService {
        _logger: Logger,
    }

    #[furnace_rs_core::cauldron]
    pub struct ConsumerCauldron;

    impl furnace_rs_core::Cauldron for ConsumerCauldron {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.provide::<LoggingService>().export::<LoggingService>()
        }
    }
}

mod default_logger_application {
    use furnace_rs_common::LoggerCauldron;

    #[furnace_rs_core::cauldron]
    pub struct ApplicationCauldron;

    impl furnace_rs_core::Cauldron for ApplicationCauldron {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.import(LoggerCauldron)
                .import(super::consumer::ConsumerCauldron)
        }
    }
}

#[tokio::test]
async fn global_logger_module_provides_the_default_logger_to_other_modules() {
    let mut builder = Furnace::builder();
    builder
        .root::<default_logger_application::ApplicationCauldron>()
        .unwrap();
    let application = builder.build().await.unwrap();

    assert!(
        application
            .context()
            .resolve::<consumer::LoggingService>()
            .is_ok()
    );
}

mod custom_logger {
    use furnace_rs_common::{LogLevel, Logger, LoggerService};

    struct TestLogger;

    impl LoggerService for TestLogger {
        fn log(&self, _level: LogLevel, _message: &str) {}
    }

    #[furnace_rs_core::cauldron]
    pub struct CustomLoggerCauldron;

    impl furnace_rs_core::Cauldron for CustomLoggerCauldron {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.provide::<Logger>().export::<Logger>().global()
        }
    }

    pub fn custom_logger() -> Logger {
        Logger::new(TestLogger)
    }
}

mod custom_logger_application {
    #[furnace_rs_core::cauldron]
    pub struct CustomLoggerApplicationCauldron;

    impl furnace_rs_core::Cauldron for CustomLoggerApplicationCauldron {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.import(super::custom_logger::CustomLoggerCauldron)
                .import(super::consumer::ConsumerCauldron)
        }
    }
}

#[tokio::test]
async fn application_can_manually_provide_a_custom_global_logger() {
    let mut builder = Furnace::builder();
    builder
        .root::<custom_logger_application::CustomLoggerApplicationCauldron>()
        .unwrap();
    builder.provide(custom_logger::custom_logger()).unwrap();
    let application = builder.build().await.unwrap();

    assert!(
        application
            .context()
            .resolve::<consumer::LoggingService>()
            .is_ok()
    );
}
