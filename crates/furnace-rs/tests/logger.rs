//! Facade coverage for the standard logger integration.

#![cfg(feature = "logger")]

use furnace_rs::{Logger, LoggerCauldron, LoggerService};

struct TestLogger;

impl LoggerService for TestLogger {
    fn log(&self, _level: furnace_rs::LogLevel, _message: &str) {}
}

#[test]
fn facade_reexports_the_logger_contract_and_global_module() {
    let _module = LoggerCauldron;
    let logger = Logger::new(TestLogger);

    logger.debug("facade logger");
}
