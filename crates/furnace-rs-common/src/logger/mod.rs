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
/// use furnace_rs_common::LoggerCauldron;
///
/// use furnace_rs_common::core::{Cauldron, CauldronRegistration};
/// #[furnace_rs_common::core::cauldron]
/// struct AppCauldron;
/// impl Cauldron for AppCauldron {
///     fn register(self) -> CauldronRegistration<Self> { self.import(LoggerCauldron) }
/// }
/// ```
///
/// To override the logger, supply a value constructed with [`Logger::new`] to
/// the application builder. Keep [`Logger`] explicitly registered in one
/// reachable cauldron and export it to its consumers. A supplied value takes
/// precedence over the linked default factory without changing ownership.
#[crate::core::cauldron]
pub struct LoggerCauldron;

/// Provides the default console-backed [`Logger`].
pub fn logger() -> Logger {
    Logger::default()
}

impl crate::core::Injector for Logger {
    type Dependencies = ();
    async fn inject((): ()) -> crate::core::Result<Self> {
        Ok(logger())
    }
    fn descriptor() -> &'static crate::core::ProviderDescriptor {
        &LOGGER_DESCRIPTOR
    }
}

const LOGGER_DESCRIPTOR: crate::core::ProviderDescriptor =
    crate::core::__private::InjectorMetadata::<Logger, Logger>::DESCRIPTOR
        .with_authored_type_name("Logger")
        .with_visibility(crate::core::ProviderVisibility::Public)
        .with_namespace(module_path!())
        .with_location(crate::core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
crate::core::__private::inventory::submit! { LOGGER_DESCRIPTOR }

impl crate::core::Cauldron for LoggerCauldron {
    fn register(self) -> crate::core::CauldronRegistration<Self> {
        self.provide::<Logger>().export::<Logger>().global()
    }
}
