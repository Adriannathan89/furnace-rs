//! Supervised HTTP upgrades and accepted connection tasks.
use super::{ApplicationRuntimeError, SocketRuntime};
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use tokio::net::TcpListener;
pub(super) async fn serve(
    listener: TcpListener,
    router: axum::Router,
    mut runtime: SocketRuntime,
) -> Result<(), ApplicationRuntimeError> {
    #[cfg(not(feature = "tls"))]
    let _ = runtime.tls;
    #[cfg(feature = "redis")]
    {
        let backend = runtime.hub.state.lock().unwrap().backend.clone();
        if let Some(backend) = backend {
            backend.start(runtime.shutdown.clone(), &runtime.tasks);
        }
    }
    loop {
        if *runtime.shutdown.borrow_and_update() {
            break;
        }
        tokio::select! {biased;
            _=runtime.shutdown.changed()=>break,
            accepted=listener.accept()=>{
                let(stream,peer)=accepted.map_err(|source|ApplicationRuntimeError::TransportServe{transport:"socket",source:Box::new(source)})?;
                let router=router.clone().layer(axum::Extension(peer)).layer(axum::Extension(runtime.clone()));
                #[allow(unused_mut)]
                let mut shutdown=runtime.shutdown.clone();
                #[cfg(feature="tls")]
                let tls=runtime.tls.clone();
                #[cfg(feature="tls")]
                let timeout=runtime.config.tls_handshake_timeout;
                runtime.tasks.spawn(async move{
                    #[cfg(feature="tls")]
                    if let Some(tls)=tls {
                        let stream=tokio::select!{biased;_=shutdown.changed()=>return,result=tokio::time::timeout(timeout,tls.accept(stream))=>match result{Ok(Ok(stream))=>stream,_=>return}};
                        connection(stream,router,shutdown).await;return;
                    }
                    connection(stream,router,shutdown).await;
                });
            }
        }
    }
    Ok(())
}
async fn connection<IO>(
    stream: IO,
    router: axum::Router,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) where
    IO: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let connection = hyper::server::conn::http1::Builder::new()
        .serve_connection(TokioIo::new(stream), TowerToHyperService::new(router))
        .with_upgrades();
    tokio::pin!(connection);
    tokio::select! {_=&mut connection=>{},_=shutdown.changed()=>{connection.as_mut().graceful_shutdown();let _=connection.await;}}
}
