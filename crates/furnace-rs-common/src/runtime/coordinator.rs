//! One owner for listener admission, drain, and application shutdown.
use super::{ApplicationRuntimeError, RuntimeTasks};
use furnace_rs_core::{Diagnostic, DiagnosticCode, Error, Furnace};
use futures_util::future::BoxFuture;
use std::{future::Future, io::IsTerminal, time::Duration};
use tokio::{net::TcpListener, sync::watch};
/// Prepared transport's supervised serving operation.
#[doc(hidden)]
pub type TransportServe = Box<
    dyn FnOnce(
            TcpListener,
            watch::Receiver<bool>,
            RuntimeTasks,
        ) -> BoxFuture<'static, Result<(), ApplicationRuntimeError>>
        + Send
        + Sync,
>;
/// Shared infrastructure readiness transition.
#[doc(hidden)]
pub type TransportTransition = Box<dyn Fn(bool) -> BoxFuture<'static, ()> + Send + Sync>;
/// Validated transport with no bound listener and no active request admission.
#[doc(hidden)]
pub struct PreparedTransport {
    /// Protocol identifier.
    pub name: &'static str,
    /// Listener address; DNS resolution is deferred until lifecycle startup succeeds.
    pub address: String,
    /// Static route/method readiness description.
    pub summary: String,
    /// Maximum graceful drain duration, when this transport requires one.
    pub drain_timeout: Option<Duration>,
    /// Infrastructure status update before opening admission and before drain.
    pub transition: Option<TransportTransition>,
    /// Native serving operation using the shared task scope.
    pub serve: TransportServe,
}
/// Runs prepared backends under one application lifecycle.
#[doc(hidden)]
pub async fn serve_prepared_transports(
    application: Furnace,
    transports: Vec<PreparedTransport>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), ApplicationRuntimeError> {
    serve_prepared_transports_with(application, transports, shutdown, |address| {
        Box::pin(TcpListener::bind(address))
    })
    .await
}
/// Runs prepared backends with an injected listener binder.
#[doc(hidden)]
pub async fn serve_prepared_transports_with<B>(
    mut application: Furnace,
    transports: Vec<PreparedTransport>,
    shutdown: impl Future<Output = ()> + Send,
    mut bind: B,
) -> Result<(), ApplicationRuntimeError>
where
    B: FnMut(String) -> BoxFuture<'static, std::io::Result<TcpListener>>,
{
    if transports.is_empty() {
        return Err(ApplicationRuntimeError::Bootstrap(Error::new(
            Diagnostic::new(
                DiagnosticCode::new("FURNACE151"),
                "no selected transport",
                "select at least one listener before starting the application",
            ),
        )));
    }
    application
        .start()
        .await
        .map_err(ApplicationRuntimeError::Lifecycle)?;
    let mut listeners = Vec::new();
    for transport in &transports {
        match bind(transport.address.clone()).await {
            Ok(listener) => listeners.push(listener),
            Err(source) => {
                drop(listeners);
                return finish(
                    application,
                    Err(ApplicationRuntimeError::TransportBind {
                        transport: transport.name,
                        source,
                    }),
                )
                .await;
            }
        }
    }
    for transport in &transports {
        if let Some(transition) = &transport.transition {
            transition(true).await;
        }
    }
    let clear = if std::io::stdout().is_terminal() {
        "\x1b[2J\x1b[H"
    } else {
        ""
    };
    println!("{clear}\n\nFURNACE · Application ready\n────────────────────────────");
    for (index, (transport, listener)) in transports.iter().zip(&listeners).enumerate() {
        if index > 0 {
            println!();
        }
        println!(
            "Server  {}://{}\n{}",
            transport.name,
            listener
                .local_addr()
                .map(|a| a.to_string())
                .unwrap_or_else(|_| transport.address.clone()),
            transport.summary
        );
    }
    println!();
    let deadline = transports.iter().filter_map(|t| t.drain_timeout).min();
    let tasks = RuntimeTasks::new();
    let (stop, stopped) = watch::channel(false);
    let mut servers = tokio::task::JoinSet::new();
    let mut transitions = Vec::new();
    for (transport, listener) in transports.into_iter().zip(listeners) {
        transitions.push(transport.transition);
        servers.spawn((transport.serve)(listener, stopped.clone(), tasks.clone()));
    }
    tokio::pin!(shutdown);
    let mut result = tokio::select! {
        biased;
        _ = &mut shutdown => Ok(()),
        result = servers.join_next() => server_result(result),
    };
    // Health changes before any shutdown signal reaches the accept loops.
    for transition in transitions.into_iter().flatten() {
        transition(false).await;
    }
    stop.send_replace(true);
    let drain = async {
        while let Some(server) = servers.join_next().await {
            let next = server_result(Some(server));
            if result.is_ok() {
                result = next;
            }
        }
        tasks.wait().await;
    };
    let expired = if let Some(deadline) = deadline {
        tokio::time::timeout(deadline, drain).await.is_err()
    } else {
        drain.await;
        false
    };
    if expired {
        servers.abort_all();
        while servers.join_next().await.is_some() {}
        tasks.abort_and_wait().await;
        if result.is_ok() {
            result = Err(ApplicationRuntimeError::DrainTimeout);
        }
    }
    finish(application, result).await
}
fn server_result(
    result: Option<Result<Result<(), ApplicationRuntimeError>, tokio::task::JoinError>>,
) -> Result<(), ApplicationRuntimeError> {
    match result {
        Some(Ok(result)) => result,
        Some(Err(source)) => Err(ApplicationRuntimeError::TransportServe {
            transport: "runtime",
            source: Box::new(source),
        }),
        None => Ok(()),
    }
}
async fn finish(
    mut application: Furnace,
    operation: Result<(), ApplicationRuntimeError>,
) -> Result<(), ApplicationRuntimeError> {
    match (operation, application.shutdown().await) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(error)) => Err(ApplicationRuntimeError::Lifecycle(error)),
        (Err(error), Ok(())) => Err(error),
        (Err(operation), Err(shutdown)) => Err(ApplicationRuntimeError::OperationAndShutdown {
            operation: Box::new(operation),
            shutdown,
        }),
    }
}
