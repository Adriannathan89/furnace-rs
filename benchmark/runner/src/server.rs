use crate::{
    config::{BenchResult, PASSWORD, SECRET, USERNAME},
    load::http_client,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}
pub fn project(case: &str) -> &str {
    match case {
        "hello" | "connection-churn" | "di-graph-noop" | "di-graph-traverse" => "hello-world",
        "posts" => "posts-crud",
        _ => "protected-route",
    }
}
fn binary_name(case: &str) -> &str {
    match project(case) {
        "hello-world" => "furnace-rs-example-hello-world",
        "posts-crud" => "furnace-rs-example-posts-crud",
        _ => "furnace-rs-example-protected-route",
    }
}
pub fn binary(framework: &str, case: &str, count: Option<usize>) -> PathBuf {
    let root = root();
    if framework.starts_with("furnace-rs") {
        let target = root.join("benchmark/targets/furnace-rs/target");
        let version = if framework == "furnace-rs-1.0.2" {
            "1.0.2"
        } else {
            "beta"
        };
        if let Some(count) = count {
            return target
                .join(format!("di-comparison/graph/{version}/nodes-{count}"))
                .join(binary_name(case));
        }
        if version == "1.0.2" {
            return target
                .join("di-comparison/baseline-1.0.2")
                .join(binary_name(case));
        }
        target.join("release").join(binary_name(case))
    } else if framework == "axum" {
        root.join("benchmark/targets/axum/target/release/furnace-rs-bench-axum")
    } else {
        root.join("benchmark/targets/fiber/bin/furnace-rs-bench-fiber")
    }
}
pub fn sha256(path: &Path) -> BenchResult<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn artifact(framework: &str, case: &str, count: Option<usize>) -> BenchResult<Value> {
    let path = binary(framework, case, count);
    Ok(
        json!({"path":path.strip_prefix(root())?,"sha256":sha256(&path)?,"bytes":std::fs::metadata(&path)?.len()}),
    )
}

pub struct Server {
    child: Child,
    pub url: String,
    pub startup_seconds: f64,
    pub log_path: PathBuf,
    successful: bool,
}
impl Server {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
    pub async fn start(framework: &str, case: &str, count: Option<usize>) -> BenchResult<Self> {
        let binary = binary(framework, case, count);
        if !binary.is_file() {
            return Err(format!(
                "missing binary {}; run furnace-bench build first",
                binary.display()
            )
            .into());
        }
        let port = match project(case) {
            "hello-world" => 3000,
            "posts-crud" => 3001,
            _ => 3002,
        };
        // Refuse to probe or stop another process already occupying the benchmark port.
        drop(
            TcpListener::bind(("127.0.0.1", port))
                .map_err(|_| format!("benchmark port {port} is already in use"))?,
        );
        let log = tempfile::NamedTempFile::with_prefix("furnace-rust-benchmark-")?;
        let stderr = log.as_file().try_clone()?;
        let (log, log_path) = log.keep()?;
        let directory = if framework.starts_with("furnace-rs") {
            root().join("example").join(project(case))
        } else {
            root().join("benchmark/targets").join(framework)
        };
        let mut command = if let Ok(cpus) = std::env::var("BENCH_SERVER_CPUS") {
            let mut command = Command::new("taskset");
            command.args(["-c", &cpus]).arg(&binary);
            command
        } else {
            Command::new(&binary)
        };
        command
            .current_dir(directory)
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(stderr))
            .env(
                "BENCH_MODE",
                if case.starts_with("di-graph-") {
                    case
                } else {
                    match project(case) {
                        "hello-world" => "hello",
                        "posts-crud" => "posts",
                        _ => "auth",
                    }
                },
            )
            .env("PORT", port.to_string())
            .env("DEMO_USERNAME", USERNAME)
            .env("DEMO_PASSWORD", PASSWORD)
            .env("JWT_SECRET", SECRET);
        if case == "posts" {
            let variable = if framework.starts_with("furnace-rs") {
                "BENCH_FURNACE_DATABASE_URL"
            } else if framework == "axum" {
                "BENCH_AXUM_DATABASE_URL"
            } else {
                "BENCH_FIBER_DATABASE_URL"
            };
            let url = std::env::var(variable)
                .or_else(|_| std::env::var("BENCH_DATABASE_URL"))
                .map_err(|_| {
                    format!("{variable} is required; use an isolated benchmark database")
                })?;
            command.env("DATABASE_URL", url);
        }
        let start = Instant::now();
        let child = command.spawn()?;
        let mut server = Self {
            child,
            url: format!("http://127.0.0.1:{port}"),
            startup_seconds: 0.0,
            log_path,
            successful: false,
        };
        let (path, status) = match project(case) {
            "posts-crud" => ("/posts", 200),
            "protected-route" => ("/auth/me", 401),
            _ => match case {
                "di-graph-noop" => ("/graph/noop", 200),
                "di-graph-traverse" => ("/graph/traverse", 200),
                _ => ("/", 200),
            },
        };
        let client = http_client()?;
        while start.elapsed() < Duration::from_secs(30) {
            if let Some(exit) = server.child.try_wait()? {
                return Err(
                    format!("server exited {exit}; log: {}", server.log_path.display()).into(),
                );
            }
            let reply = client
                .get(format!("{}{path}", server.url))
                .timeout(Duration::from_millis(200))
                .send()
                .await;
            if let Ok(response) = reply
                && response.status().as_u16() == status
                && response.bytes().await.is_ok()
            {
                server.startup_seconds = start.elapsed().as_secs_f64();
                return Ok(server);
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        Err(format!(
            "server startup timed out; log: {}",
            server.log_path.display()
        )
        .into())
    }
    pub async fn stop(&mut self, successful: bool) -> BenchResult<()> {
        if self.child.try_wait()?.is_none() {
            #[cfg(unix)]
            {
                let _ = Command::new("kill")
                    .args(["-INT", &self.child.id().to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                let deadline = Instant::now() + Duration::from_secs(5);
                while Instant::now() < deadline && self.child.try_wait()?.is_none() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
            if self.child.try_wait()?.is_none() {
                self.child.kill()?;
            }
        }
        self.child.wait()?;
        self.successful = successful;
        Ok(())
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        if self.successful {
            let _ = std::fs::remove_file(&self.log_path);
        }
    }
}

pub fn build(frameworks: &[String]) -> BenchResult<()> {
    for framework in frameworks {
        if framework == "furnace-rs-1.0.2" {
            return Err(
                "baseline binaries require the historical build_di_comparison.py build helper"
                    .into(),
            );
        }
        if framework.starts_with("furnace-rs") {
            for example in ["hello-world", "protected-route", "posts-crud"] {
                let status = Command::new("cargo")
                    .current_dir(root())
                    .args(["build", "--release", "--locked", "--manifest-path"])
                    .arg(root().join(format!("example/{example}/Cargo.toml")))
                    .arg("--target-dir")
                    .arg(root().join("benchmark/targets/furnace-rs/target"))
                    .status()?;
                if !status.success() {
                    return Err(format!("{example} build failed").into());
                }
            }
        } else if framework == "axum" {
            let status = Command::new("cargo")
                .current_dir(root())
                .args(["build", "--release", "--locked", "--manifest-path"])
                .arg(root().join("benchmark/targets/axum/Cargo.toml"))
                .status()?;
            if !status.success() {
                return Err("Axum build failed".into());
            }
        } else {
            let directory = root().join("benchmark/targets/fiber");
            std::fs::create_dir_all(directory.join("bin"))?;
            let status = Command::new("go")
                .current_dir(&directory)
                .args(["build", "-o"])
                .arg(directory.join("bin/furnace-rs-bench-fiber"))
                .arg(".")
                .status()?;
            if !status.success() {
                return Err("Fiber build failed".into());
            }
        }
    }
    Ok(())
}
