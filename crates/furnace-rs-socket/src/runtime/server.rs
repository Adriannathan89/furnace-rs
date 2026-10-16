//! Supervised HTTP upgrades and accepted connection tasks.
use super::{ApplicationRuntimeError, SocketRuntime};
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use tokio::net::TcpListener;
/// Serves until shared admission closes; upgraded tasks remain runtime-owned.
pub(super) async fn serve(
    listener: TcpListener,
    router: axum::Router,
    mut runtime: SocketRuntime,
) -> Result<(), ApplicationRuntimeError> {
    loop {
        if *runtime.shutdown.borrow_and_update() {
            break;
        }
        tokio::select! {biased;
            _=runtime.shutdown.changed()=>break,
            accepted=listener.accept()=>{
                let (stream,peer)=accepted.map_err(|source|ApplicationRuntimeError::TransportServe{transport:"socket",source:Box::new(source)})?;
                let router=router.clone().layer(axum::Extension(peer)).layer(axum::Extension(runtime.clone()));
                let mut shutdown=runtime.shutdown.clone();
                runtime.tasks.spawn(async move {
                    let connection=hyper::server::conn::http1::Builder::new().serve_connection(TokioIo::new(stream),TowerToHyperService::new(router)).with_upgrades();
                    tokio::pin!(connection);
                    tokio::select! {
                        _=&mut connection=>{},
                        _=shutdown.changed()=>{connection.as_mut().graceful_shutdown();let _=connection.await;}
                    }
                });
            }
        }
    }
    Ok(())
}
