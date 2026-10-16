#![allow(missing_docs, dead_code)]
//! Test-only listener ownership and client helpers.
use furnace_rs_core::{Cauldron, ConfigBuilder, Furnace, MapSource};
use furnace_rs_socket::runtime::{
    RuntimeBindings, RuntimeRoutes, RuntimeSocketExt, serve_transports,
};
use std::time::Duration;
pub struct Server {
    pub address: std::net::SocketAddr,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: Option<
        tokio::task::JoinHandle<Result<(), furnace_rs_socket::runtime::ApplicationRuntimeError>>,
    >,
}
impl Server {
    pub async fn start<M: Cauldron>() -> Self {
        Self::configured::<M>(&[]).await
    }
    pub async fn configured<M: Cauldron>(extra: &[(&str, &str)]) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let mut values = vec![("socket.address".to_string(), address.to_string())];
        values.extend(extra.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        let config = ConfigBuilder::new()
            .source(MapSource::new("test", values))
            .build()
            .unwrap();
        let mut b = Furnace::builder_with_config(config.clone());
        b.root::<M>().unwrap();
        let app = b.build().await.unwrap();
        let router = furnace_rs_socket::build_socket_routes(&app).unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(serve_transports(
            app,
            RuntimeRoutes::new().with_socket(router),
            RuntimeBindings::new()
                .with_socket(furnace_rs_socket::SocketServerConfig::parse(&config).unwrap()),
            async {
                let _ = stopped.await;
            },
        ));
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if tokio::net::TcpStream::connect(address).await.is_ok() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        Self {
            address,
            stop: Some(stop),
            task: Some(task),
        }
    }
    pub async fn connect(
        &self,
        path: &str,
    ) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
    {
        tokio_tungstenite::connect_async(format!("ws://{}{path}", self.address))
            .await
            .unwrap()
            .0
    }
    pub async fn stop(mut self) {
        let _ = self.stop.take().unwrap().send(());
        let result = tokio::time::timeout(Duration::from_secs(12), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_ok(), "{result:?}");
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(s) = self.stop.take() {
            let _ = s.send(());
        }
        if let Some(t) = self.task.take() {
            t.abort();
        }
    }
}
