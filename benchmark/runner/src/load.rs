use crate::{
    config::{BenchResult, PASSWORD, Random, USERNAME},
    metrics::{Measurement, Measurements, cpu_delta, percentile, process_cpu_seconds},
};
use bytes::Bytes;
use reqwest::{Client, Method, header::HeaderMap};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{io::AsyncWriteExt, sync::Barrier, task::JoinSet};

pub fn http_client() -> BenchResult<Client> {
    client_with_idle_pool(1)
}

fn client_with_idle_pool(limit: usize) -> BenchResult<Client> {
    Ok(Client::builder()
        .http1_only()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(2))
        .tcp_nodelay(true)
        .pool_max_idle_per_host(limit)
        .build()?)
}

pub fn sized_json(size: usize) -> Bytes {
    let prefix = b"{\"username\":\"";
    let suffix = b"\",\"password\":\"short\"}";
    let mut body = Vec::with_capacity(size);
    body.extend_from_slice(prefix);
    body.resize(size - suffix.len(), b'x');
    body.extend_from_slice(suffix);
    body.into()
}

#[derive(Clone)]
pub struct LoadConfig {
    pub case: String,
    pub url: String,
    pub operations: usize,
    pub concurrency: usize,
    pub seed: u64,
    pub dependency_count: Option<usize>,
    pub server_pid: Option<u32>,
}
struct Context {
    load: LoadConfig,
    login: Bytes,
    large: Bytes,
    boundaries: Vec<Bytes>,
    token: Option<String>,
}

pub struct Reply {
    pub status: u16,
    pub body: Bytes,
    pub headers: HeaderMap,
}
pub enum Check {
    Any,
    Text(String),
    Json(Value),
    Error(&'static str),
}

#[expect(
    clippy::too_many_arguments,
    reason = "explicit wire inputs and response contracts keep benchmark cases auditable"
)]
pub async fn request(
    client: &Client,
    stats: &mut Measurements,
    base: &str,
    method: Method,
    path: &str,
    body: Option<Bytes>,
    authorization: Option<&str>,
    status: u16,
    check: Check,
) -> Option<Reply> {
    stats.attempted_requests += 1;
    let started = Instant::now();
    let result: BenchResult<Reply> = async {
        let mut builder = client.request(method, format!("{}{path}", base.trim_end_matches('/')));
        if let Some(body) = body {
            builder = builder
                .header("content-type", "application/json")
                .body(body);
        }
        if let Some(authorization) = authorization {
            builder = builder.header("authorization", authorization);
        }
        let response = builder.send().await?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response.bytes().await?;
        Ok(Reply {
            status,
            headers,
            body,
        })
    }
    .await;
    match result {
        Ok(reply) => {
            stats
                .latencies
                .push(started.elapsed().as_secs_f64() * 1000.0);
            *stats.statuses.entry(reply.status).or_default() += 1;
            if reply.status != status {
                stats.error(format!(
                    "{path}: expected HTTP {status}, received {}",
                    reply.status
                ));
            } else {
                let valid = match check {
                    Check::Any => true,
                    Check::Text(expected) => reply.body.as_ref() == expected.as_bytes(),
                    Check::Json(expected) => {
                        serde_json::from_slice::<Value>(&reply.body).is_ok_and(|v| v == expected)
                    }
                    Check::Error(code) => serde_json::from_slice::<Value>(&reply.body)
                        .is_ok_and(|v| v["error"]["code"] == code),
                };
                if !valid {
                    stats.error(format!(
                        "{path}: response body did not match the expected contract"
                    ));
                }
            }
            Some(reply)
        }
        Err(error) => {
            stats.error(format!("{path}: transport failure: {error}"));
            None
        }
    }
}

fn token(reply: Option<Reply>, stats: &mut Measurements) -> Option<String> {
    let reply = reply?;
    if reply.status != 200 {
        return None;
    }
    let value = serde_json::from_slice::<Value>(&reply.body).ok();
    if let Some(token) = value
        .as_ref()
        .and_then(|v| v["access_token"].as_str())
        .filter(|v| !v.is_empty())
    {
        return Some(token.to_owned());
    }
    stats.error("login did not return a nonempty access_token");
    None
}

fn close_header(reply: &Reply, stats: &mut Measurements) {
    if !reply
        .headers
        .get("connection")
        .is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"close"))
    {
        stats.error("413 response did not signal Connection: close");
    }
}
fn challenge(reply: &Reply, stats: &mut Measurements) {
    if reply.headers.get("www-authenticate").map(|v| v.as_bytes()) != Some(b"Bearer") {
        stats.error("401 response did not include WWW-Authenticate: Bearer");
    }
}

impl Context {
    async fn operation(
        &self,
        client: &Client,
        stats: &mut Measurements,
        worker: usize,
        index: usize,
    ) {
        let base = &self.load.url;
        match self.load.case.as_str() {
            "hello" | "connection-churn" | "di-graph-noop" | "di-graph-traverse" => {
                let (path, expected) = match self.load.case.as_str() {
                    "di-graph-noop" => ("/graph/noop", "ok".to_owned()),
                    "di-graph-traverse" => (
                        "/graph/traverse",
                        self.load.dependency_count.unwrap().to_string(),
                    ),
                    _ => ("/", "Hello, world!".to_owned()),
                };
                request(
                    client,
                    stats,
                    base,
                    Method::GET,
                    path,
                    None,
                    None,
                    200,
                    Check::Text(expected),
                )
                .await;
            }

            "auth" | "aborted-upload" => {
                if self.load.case == "aborted-upload" {
                    let result: BenchResult<()> = async {
                        let url = reqwest::Url::parse(base)?;
                        let mut stream = tokio::time::timeout(Duration::from_secs(10), tokio::net::TcpStream::connect((url.host_str().ok_or("missing host")?, url.port_or_known_default().ok_or("missing port")?))).await??;
                        let mut partial = b"POST /auth/login HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 1048576\r\n\r\n".to_vec();
                        partial.extend_from_slice(&[b'x';1024]);
                        tokio::time::timeout(Duration::from_secs(10), stream.write_all(&partial)).await??;
                        stream.shutdown().await?;
                        Ok(())
                    }.await;
                    if let Err(error) = result {
                        stats.error(format!("aborted upload: {error}"));
                    }
                }
                let login = request(
                    client,
                    stats,
                    base,
                    Method::POST,
                    "/auth/login",
                    Some(self.login.clone()),
                    None,
                    200,
                    Check::Any,
                )
                .await;
                let Some(token) = token(login, stats) else {
                    return;
                };
                request(
                    client,
                    stats,
                    base,
                    Method::GET,
                    "/auth/me",
                    None,
                    Some(&format!("Bearer {token}")),
                    200,
                    Check::Json(json!({"id":1,"username":USERNAME})),
                )
                .await;
            }
            "jwt" => {
                let authorization = match index % 3 {
                    0 => Some(format!("Bearer {}", self.token.as_ref().unwrap())),
                    1 => None,
                    _ => Some("Bearer bad.token.value".into()),
                };
                let valid = index.is_multiple_of(3);
                let reply = request(
                    client,
                    stats,
                    base,
                    Method::GET,
                    "/auth/me",
                    None,
                    authorization.as_deref(),
                    if valid { 200 } else { 401 },
                    if valid {
                        Check::Json(json!({"id":1,"username":USERNAME}))
                    } else {
                        Check::Error("unauthorized")
                    },
                )
                .await;
                if !valid
                    && let Some(reply) = reply
                    && reply.status == 401
                {
                    challenge(&reply, stats);
                }
            }
            "validation" => {
                let body = match index % 3 {
                    0 => Bytes::from_static(b"{\"username\":\"\",\"password\":\"short\"}"),
                    1 => Bytes::from_static(b"{"),
                    _ => self.large.clone(),
                };
                request(
                    client,
                    stats,
                    base,
                    Method::POST,
                    "/auth/login",
                    Some(body),
                    None,
                    422,
                    Check::Error("validation_error"),
                )
                .await;
            }
            "oversized" | "oversized-reuse" => {
                let reply = request(
                    client,
                    stats,
                    base,
                    Method::POST,
                    "/auth/login",
                    Some(self.large.clone()),
                    None,
                    413,
                    Check::Error("payload_too_large"),
                )
                .await;
                if self.load.case == "oversized-reuse"
                    && let Some(reply) = reply
                    && reply.status == 413
                {
                    close_header(&reply, stats);
                }
            }
            "body-limit-boundary" => {
                let variant = (worker + index) % 3;
                let reply = request(
                    client,
                    stats,
                    base,
                    Method::POST,
                    "/auth/login",
                    Some(self.boundaries[variant].clone()),
                    None,
                    if variant == 2 { 413 } else { 422 },
                    Check::Error(if variant == 2 {
                        "payload_too_large"
                    } else {
                        "validation_error"
                    }),
                )
                .await;
                if let Some(reply) = reply
                    && reply.status == 413
                {
                    close_header(&reply, stats);
                }
                let reply = request(
                    client,
                    stats,
                    base,
                    Method::GET,
                    "/auth/me",
                    None,
                    None,
                    401,
                    Check::Error("unauthorized"),
                )
                .await;
                if let Some(reply) = reply
                    && reply.status == 401
                {
                    challenge(&reply, stats);
                }
            }
            "posts" => {
                let title = format!("bench-{worker}-{index}");
                let created = request(
                    client,
                    stats,
                    base,
                    Method::POST,
                    "/posts",
                    Some(json!({"title":title,"body":"before"}).to_string().into()),
                    None,
                    201,
                    Check::Any,
                )
                .await;
                let Some(created) = created.filter(|v| v.status == 201) else {
                    return;
                };
                let Ok(post) = serde_json::from_slice::<Value>(&created.body) else {
                    stats.error("invalid create JSON");
                    return;
                };
                let Some(id) = post["id"].as_i64() else {
                    stats.error("create did not return an integer id");
                    return;
                };
                if post != json!({"id":id,"title":title,"body":"before"}) {
                    stats.error("create content differs");
                }
                let path = format!("/posts/{id}");
                request(
                    client,
                    stats,
                    base,
                    Method::GET,
                    &path,
                    None,
                    None,
                    200,
                    Check::Json(json!({"id":id,"title":title,"body":"before"})),
                )
                .await;
                request(
                    client,
                    stats,
                    base,
                    Method::PUT,
                    &path,
                    Some(json!({"title":title,"body":"after"}).to_string().into()),
                    None,
                    200,
                    Check::Json(json!({"id":id,"title":title,"body":"after"})),
                )
                .await;
                request(
                    client,
                    stats,
                    base,
                    Method::DELETE,
                    &path,
                    None,
                    None,
                    204,
                    Check::Text(String::new()),
                )
                .await;
                request(
                    client,
                    stats,
                    base,
                    Method::GET,
                    &path,
                    None,
                    None,
                    404,
                    Check::Error("not_found"),
                )
                .await;
            }
            _ => unreachable!("validated scenario"),
        }
    }
}

pub async fn measure(load: LoadConfig) -> BenchResult<Measurement> {
    let url = reqwest::Url::parse(&load.url)?;
    if url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("benchmark URL must use HTTP without credentials, query, or fragment".into());
    }
    let mut context = Context {
        load,
        login: json!({"username":USERNAME,"password":PASSWORD})
            .to_string()
            .into(),
        large: Bytes::new(),
        boundaries: vec![],
        token: None,
    };
    match context.load.case.as_str() {
        "validation" => context.large = sized_json(128 * 1024 + 34),
        "oversized" | "oversized-reuse" => context.large = sized_json(3 * 1024 * 1024 + 34),
        "body-limit-boundary" => {
            context.boundaries = (2_097_151..=2_097_153).map(sized_json).collect()
        }
        "jwt" => {
            let mut stats = Measurements::default();
            let login = request(
                &http_client()?,
                &mut stats,
                &context.load.url,
                Method::POST,
                "/auth/login",
                Some(context.login.clone()),
                None,
                200,
                Check::Any,
            )
            .await;
            context.token = token(login, &mut stats);
            if stats.error_count != 0 || context.token.is_none() {
                return Err("JWT setup failed".into());
            }
        }
        _ => {}
    }
    let context = Arc::new(context);
    let operations = context.load.operations;
    let concurrency = context.load.concurrency.min(operations);
    if operations == 0 || concurrency == 0 {
        return Err("operations and concurrency must be positive".into());
    }
    let barrier = Arc::new(Barrier::new(concurrency + 1));
    let mut workers = JoinSet::new();
    let mut order_hash = Sha256::new();
    let mut first_order_entry = true;
    for worker in 0..concurrency {
        let count = operations / concurrency + usize::from(worker < operations % concurrency);
        let mut order: Vec<usize> = (0..count).collect();
        Random(context.load.seed ^ (worker as u64).wrapping_mul(0x9e3779b1)).shuffle(&mut order);
        for index in &order {
            if !first_order_entry {
                order_hash.update(b";");
            }
            first_order_entry = false;
            order_hash.update(format!("{worker}:{index}"));
        }
        let context = context.clone();
        let barrier = barrier.clone();
        let client = client_with_idle_pool(if context.load.case == "connection-churn" {
            0
        } else {
            1
        })?;
        workers.spawn(async move {
            let mut stats = Measurements::default();
            stats
                .latencies
                .reserve(count * expected_responses(&context.load.case));
            barrier.wait().await;
            for index in order {
                let errors = stats.error_count;
                let responses = stats.latencies.len();
                context.operation(&client, &mut stats, worker, index).await;
                if errors == stats.error_count
                    && stats.latencies.len() - responses == expected_responses(&context.load.case)
                {
                    stats.completed_operations += 1;
                }
            }
            stats
        });
    }
    let before = process_cpu_seconds(std::process::id());
    let server_before = context.load.server_pid.and_then(process_cpu_seconds);
    let start = Instant::now();
    barrier.wait().await;
    let mut stats = Measurements::default();
    while let Some(worker) = workers.join_next().await {
        stats.merge(worker?);
    }
    let seconds = start.elapsed().as_secs_f64();
    let cpu = cpu_delta(before, process_cpu_seconds(std::process::id()));
    let server_cpu = cpu_delta(
        server_before,
        context.load.server_pid.and_then(process_cpu_seconds),
    );
    stats.latencies.sort_unstable_by(f64::total_cmp);
    let responses = stats.latencies.len();
    let expected = operations
        .checked_mul(expected_responses(&context.load.case))
        .ok_or("response count overflow")?;
    if responses != expected {
        stats.error(format!(
            "incomplete responses: expected {expected}, received {responses}"
        ));
    }
    let passed = stats.error_count == 0 && stats.completed_operations == operations;
    Ok(Measurement {
        case: context.load.case.clone(),
        operations,
        completed_operations: stats.completed_operations,
        concurrency,
        attempted_requests: stats.attempted_requests,
        http_responses: responses,
        status_counts: stats.statuses,
        duration_seconds: seconds,
        responses_per_second: responses as f64 / seconds,
        p50_ms: percentile(&stats.latencies, 50),
        p95_ms: percentile(&stats.latencies, 95),
        p99_ms: percentile(&stats.latencies, 99),
        client_cpu_seconds: cpu,
        client_cpu_percent_one_core: cpu.map(|v| v / seconds * 100.0),
        server_cpu_seconds: server_cpu,
        server_cpu_percent_one_core: server_cpu.map(|v| v / seconds * 100.0),
        request_order_seed: context.load.seed,
        request_order_sha256: format!("{:x}", order_hash.finalize()),
        error_count: stats.error_count,
        error_examples: stats.error_examples,
        passed,
    })
}
pub fn expected_responses(case: &str) -> usize {
    match case {
        "auth" | "aborted-upload" | "body-limit-boundary" => 2,
        "posts" => 5,
        _ => 1,
    }
}

pub async fn contracts(case: &str, base: &str, count: Option<usize>) -> BenchResult<Vec<String>> {
    let client = http_client()?;
    let mut stats = Measurements::default();
    if case == "auth"
        || case == "jwt"
        || [
            "validation",
            "oversized",
            "oversized-reuse",
            "body-limit-boundary",
            "aborted-upload",
        ]
        .contains(&case)
    {
        for auth in [None, Some("Bearer bad.token.value")] {
            let reply = request(&client,&mut stats,base,Method::GET,"/auth/me",None,auth,401,Check::Json(json!({"error":{"code":"unauthorized","message":"authentication was rejected"}}))).await;
            if let Some(reply) = reply
                && reply.status == 401
            {
                challenge(&reply, &mut stats);
            }
        }
        request(
            &client,
            &mut stats,
            base,
            Method::POST,
            "/auth/login",
            Some(Bytes::from_static(
                b"{\"username\":\"\",\"password\":\"short\"}",
            )),
            None,
            422,
            Check::Error("validation_error"),
        )
        .await;
        request(
            &client,
            &mut stats,
            base,
            Method::POST,
            "/auth/login",
            Some(Bytes::from_static(b"{")),
            None,
            422,
            Check::Error("validation_error"),
        )
        .await;
        let unsupported = client
            .post(format!("{base}/auth/login"))
            .header("content-type", "text/plain")
            .body("{}")
            .send()
            .await?;
        let status = unsupported.status();
        let body: Value = serde_json::from_slice(&unsupported.bytes().await?)?;
        if status.as_u16() != 415 || body["error"]["code"] != "unsupported_media_type" {
            stats.error("unsupported content type contract differs");
        }
        let login = request(
            &client,
            &mut stats,
            base,
            Method::POST,
            "/auth/login",
            Some(
                json!({"username":USERNAME,"password":PASSWORD})
                    .to_string()
                    .into(),
            ),
            None,
            200,
            Check::Any,
        )
        .await;
        if let Some(token) = token(login, &mut stats) {
            request(
                &client,
                &mut stats,
                base,
                Method::GET,
                "/auth/me",
                None,
                Some(&format!("bearer {token}")),
                200,
                Check::Json(json!({"id":1,"username":USERNAME})),
            )
            .await;
        }
    } else if case == "posts" {
        request(
            &client,
            &mut stats,
            base,
            Method::GET,
            "/posts/2147483647",
            None,
            None,
            404,
            Check::Error("not_found"),
        )
        .await;
        request(
            &client,
            &mut stats,
            base,
            Method::POST,
            "/posts",
            Some(json!({"title":"","body":"x"}).to_string().into()),
            None,
            422,
            Check::Error("validation_error"),
        )
        .await;
        let response = client
            .post(format!("{base}/posts"))
            .header("content-type", "text/plain")
            .body("{}")
            .send()
            .await?;
        let status = response.status();
        let body: Value = serde_json::from_slice(&response.bytes().await?)?;
        if status.as_u16() != 415 || body["error"]["code"] != "unsupported_media_type" {
            stats.error("unsupported post content type contract differs");
        }
    } else {
        let (path, body) = match case {
            "di-graph-noop" => ("/graph/noop", "ok".into()),
            "di-graph-traverse" => (
                "/graph/traverse",
                count.ok_or("missing dependency count")?.to_string(),
            ),
            _ => ("/", "Hello, world!".into()),
        };
        request(
            &client,
            &mut stats,
            base,
            Method::GET,
            path,
            None,
            None,
            200,
            Check::Text(body),
        )
        .await;
    }
    Ok(stats.error_examples)
}

pub async fn body_limit_probe(base: &str) -> Value {
    let result: BenchResult<Value> = async {
        let response = http_client()?
            .post(format!("{base}/auth/login"))
            .header("content-type", "application/json")
            .body(sized_json(3 * 1024 * 1024 + 34))
            .send()
            .await?;
        let status = response.status().as_u16();
        let _ = response.bytes().await?;
        Ok(json!({"status":status,"transport_errors":[]}))
    }
    .await;
    match result {
        Ok(value) => value,
        Err(error) => json!({"status":null,"transport_errors":[error.to_string()]}),
    }
}
