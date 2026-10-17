//! Isolated real Redis process owned by an integration test.
#![allow(dead_code)]
pub struct Redis {
    child: std::process::Child,
    _dir: tempfile::TempDir,
    pub url: String,
}
impl Redis {
    pub async fn start() -> Self {
        let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let dir = tempfile::tempdir().unwrap();
        let child = std::process::Command::new("redis-server")
            .args([
                "--bind",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--save",
                "",
                "--appendonly",
                "no",
                "--dir",
            ])
            .arg(dir.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Redis integration requires redis-server");
        let server = Self {
            child,
            _dir: dir,
            url: format!("redis://127.0.0.1:{port}"),
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if let Ok(mut c) = redis::Client::open(server.url.as_str())
                    .unwrap()
                    .get_multiplexed_async_connection()
                    .await
                    && redis::cmd("PING")
                        .query_async::<String>(&mut c)
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
        server
    }
}
impl Drop for Redis {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
