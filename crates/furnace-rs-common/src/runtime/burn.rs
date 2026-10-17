//! Conventional source loading and selection before provider construction.
use super::{
    ApplicationRuntimeError, RuntimeSelection, RuntimeTransportDescriptor,
    serve_prepared_transports,
};
use furnace_rs_core::{
    __private::ModuleContract, Config, ConfigBuilder, DotenvSource, EnvSource, Furnace, TomlSource,
};
use std::{future::Future, path::Path};
/// Conventional automatic startup for the configured application transports.
pub trait FurnaceBurnExt {
    /// Loads conventional configuration and runs the rooted application.
    fn burn<M: ModuleContract>() -> impl Future<Output = Result<(), ApplicationRuntimeError>> + Send;
}
impl FurnaceBurnExt for Furnace {
    #[allow(clippy::manual_async_fn)] // Explicit Send promise on the public trait.
    /// Loads conventional settings, prepares selected transports, and runs their shared lifecycle.
    fn burn<M: ModuleContract>() -> impl Future<Output = Result<(), ApplicationRuntimeError>> + Send
    {
        async move {
            let root = std::env::current_dir().map_err(|source| {
                ApplicationRuntimeError::Bootstrap(furnace_rs_core::Error::with_source(
                    furnace_rs_core::Diagnostic::new(
                        furnace_rs_core::FURNACE020,
                        "configuration directory unavailable",
                        "could not determine the current directory",
                    ),
                    source,
                ))
            })?;
            // The inspection path is static and never reaches preparation/startup.
            if let Some(result) = crate::inspection::try_run_inspection::<M>(&root) {
                return result;
            }
            let config = load_standard_config(&root).map_err(ApplicationRuntimeError::Bootstrap)?;
            let (application, transports) = prepare_application::<M>(config).await?;
            serve_prepared_transports(application, transports, shutdown_signal()).await
        }
    }
}
/// Loads optional .env and furnace.toml, then FURNACE_* environment overrides.
#[doc(hidden)]
pub fn load_standard_config(root: &Path) -> furnace_rs_core::Result<Config> {
    ConfigBuilder::new()
        .dotenv(DotenvSource::optional(root.join(".env")))
        .source(TomlSource::optional(root.join("furnace.toml")))
        .source(EnvSource::new("FURNACE_"))
        .build()
}
/// Prepares selected automatic listeners without starting lifecycle or binding.
#[doc(hidden)]
pub async fn prepare_application<M: ModuleContract>(
    config: Config,
) -> Result<(Furnace, Vec<super::PreparedTransport>), ApplicationRuntimeError> {
    let adapters: Vec<_> =
        furnace_rs_core::__private::inventory::iter::<RuntimeTransportDescriptor>
            .into_iter()
            .collect();
    let selection = RuntimeSelection::parse_registered(
        &config,
        &adapters.iter().map(|a| a.name).collect::<Vec<_>>(),
    )
    .map_err(ApplicationRuntimeError::Bootstrap)?;
    let selected = |name| match name {
        "http" => selection.http,
        "grpc" => selection.grpc,
        "socket" => selection.socket,
        _ => false,
    };
    let graph = furnace_rs_core::__private::build_cauldron_graph::<M>()
        .map_err(ApplicationRuntimeError::Bootstrap)?;
    for adapter in &adapters {
        if selected(adapter.name) {
            (adapter.validate)(&graph, &config).map_err(ApplicationRuntimeError::Bootstrap)?;
        }
    }
    let mut builder = Furnace::builder_with_config(config);
    builder
        .root::<M>()
        .map_err(ApplicationRuntimeError::Bootstrap)?;
    for adapter in &adapters {
        if selected(adapter.name) {
            (adapter.configure)(&mut builder);
        }
    }
    let application = builder
        .build()
        .await
        .map_err(ApplicationRuntimeError::Bootstrap)?;
    let mut transports = Vec::new();
    for adapter in adapters {
        if selected(adapter.name) {
            transports.push((adapter.prepare)(&application, None, None).await?);
        }
    }
    Ok((application, transports))
}
/// Waits for OS termination or the existing development shutdown file.
#[doc(hidden)]
pub async fn shutdown_signal() {
    let os_signal = async {
        #[cfg(unix)]
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = signal.recv() => {} }
            return;
        }
        let _ = tokio::signal::ctrl_c().await;
    };
    let Some(path) = std::env::var_os("FURNACE_INTERNAL_DEV_SHUTDOWN_PATH") else {
        os_signal.await;
        return;
    };
    let file_signal = async move {
        loop {
            if tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file()) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    };
    tokio::select! { _ = os_signal => {}, _ = file_signal => {} }
}
