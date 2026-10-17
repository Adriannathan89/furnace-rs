//! Independent consumer tests exercising the example's public registration API.
use furnace::prelude::*;
use furnace_rs_example_socket_transport::App;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
struct Server {
    address: std::net::SocketAddr,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<(), ApplicationRuntimeError>>>,
}
impl Server {
    async fn start(redis: Option<&str>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let mut values = vec![
            ("socket.address".to_owned(), address.to_string()),
            ("socket.namespace".into(), "example-test".into()),
            (
                "passport.secret".into(),
                "example-test-secret-with-more-than-thirty-two-characters".into(),
            ),
        ];
        if let Some(redis) = redis {
            values.push(("socket.redis.url".into(), redis.into()));
            values.push(("demo.replay".into(), "true".into()));
        }
        let config = furnace::core::ConfigBuilder::new()
            .source(furnace::core::MapSource::new("test", values))
            .build()
            .unwrap();
        let mut builder = Furnace::builder_with_config(config.clone());
        builder.root::<App>().unwrap();
        let app = builder.build().await.unwrap();
        let routes = furnace::socket::build_socket_routes(&app).unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(serve_transports(
            app,
            RuntimeRoutes::new().with_socket(routes),
            RuntimeBindings::new().with_socket(SocketServerConfig::parse(&config).unwrap()),
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
    async fn connect(&self, path: &str) -> Client {
        #[allow(unused_mut)]
        let mut url = format!("ws://{}{path}", self.address);
        #[cfg(feature = "jwt")]
        if path == "/chat" {
            let config = furnace::core::ConfigBuilder::new()
                .source(furnace::core::MapSource::new(
                    "test",
                    [(
                        "passport.secret",
                        "example-test-secret-with-more-than-thirty-two-characters",
                    )],
                ))
                .build()
                .unwrap();
            let jwt = furnace::JwtService::from_config(&config).unwrap();
            let token = jwt
                .sign(
                    furnace_rs_example_socket_transport::Claims { id: 1 },
                    furnace::JwtSignOptions::access(Duration::from_secs(60)),
                )
                .unwrap();
            url.push_str(&format!("?access_token={token}"));
        }
        tokio_tungstenite::connect_async(url).await.unwrap().0
    }
    async fn stop(mut self) {
        let _ = self.stop.take().unwrap().send(());
        assert!(self.task.take().unwrap().await.unwrap().is_ok());
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
type Client =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
async fn send(c: &mut Client, event: &str, data: Value) {
    c.send(tokio_tungstenite::tungstenite::Message::Text(
        json!({"event":event,"data":data}).to_string().into(),
    ))
    .await
    .unwrap();
}
async fn recv(c: &mut Client) -> Value {
    serde_json::from_str(
        &tokio::time::timeout(Duration::from_secs(3), c.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn example_serves_owned_raw_and_authorized_chat() {
    let s = Server::start(None).await;
    for path in ["/raw", "/owned"] {
        let mut c = s.connect(path).await;
        c.send(tokio_tungstenite::tungstenite::Message::Text(
            "hello".into(),
        ))
        .await
        .unwrap();
        assert_eq!(
            c.next().await.unwrap().unwrap().into_text().unwrap(),
            "hello"
        );
        c.close(None).await.unwrap();
    }
    let mut c = s.connect("/chat").await;
    send(&mut c, "furnace.join", json!({"room":"lobby"})).await;
    assert_eq!(recv(&mut c).await["event"], "furnace.ok");
    send(&mut c, "chat.send", json!({"text":"hello"})).await;
    assert_eq!(recv(&mut c).await["data"]["text"], "hello");
    c.close(None).await.unwrap();
    s.stop().await;
}
#[cfg(feature = "redis")]
#[tokio::test]
async fn example_resumes_on_another_instance() {
    struct Redis(std::process::Child);
    impl Drop for Redis {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let _redis = Redis(
        std::process::Command::new("redis-server")
            .args([
                "--bind",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--save",
                "",
                "--appendonly",
                "no",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let url = format!("redis://127.0.0.1:{port}");
    let a = Server::start(Some(&url)).await;
    let b = Server::start(Some(&url)).await;
    let mut ca = a.connect("/chat").await;
    send(&mut ca, "furnace.join", json!({"room":"lobby"})).await;
    assert_eq!(recv(&mut ca).await["event"], "furnace.ok");
    send(&mut ca, "chat.send", json!(1)).await;
    let cursor = recv(&mut ca).await["cursor"].as_str().unwrap().to_owned();
    send(&mut ca, "chat.send", json!(2)).await;
    assert_eq!(recv(&mut ca).await["data"], 2);
    let mut cb = b.connect("/chat").await;
    send(
        &mut cb,
        "furnace.join",
        json!({"room":"lobby","after":cursor}),
    )
    .await;
    assert_eq!(recv(&mut cb).await["data"], 2);
    assert_eq!(recv(&mut cb).await["event"], "furnace.ok");
    ca.close(None).await.unwrap();
    cb.close(None).await.unwrap();
    a.stop().await;
    b.stop().await;
}
