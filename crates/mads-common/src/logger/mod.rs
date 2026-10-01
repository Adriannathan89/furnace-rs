mod console_logger_service;
mod logger_service;

pub use console_logger_service::ConsoleLoggerService;
pub use logger_service::{LogLevel, Logger, LoggerService};

/// Global module that provides the default console-backed [`Logger`].
///
/// Import this module from the application's root module when the default
/// [`ConsoleLoggerService`] is wanted:
///
/// ```
/// use mads_common::LoggerModule;
///
/// use mads_common::core::{Furnace, FurnaceRegistration};
/// #[mads_common::core::furnace]
/// struct AppModule;
/// impl Furnace for AppModule {
///     fn register(self) -> FurnaceRegistration<Self> { self.import(LoggerModule) }
/// }
/// ```
///
/// To override the logger, supply a value constructed with [`Logger::new`] to
/// the application builder. Keep [`Logger`] explicitly registered in one
/// reachable furnace and export it to its consumers. A supplied value takes
/// precedence over the linked default factory without changing ownership.
#[crate::core::furnace]
pub struct LoggerModule;

/// Provides the default console-backed [`Logger`].
#[crate::core::element]
pub fn logger() -> Logger {
    Logger::default()
}

impl crate::core::Furnace for LoggerModule {
    fn register(self) -> crate::core::FurnaceRegistration<Self> {
        self.provide::<Logger>().export::<Logger>().global()
    }
}
