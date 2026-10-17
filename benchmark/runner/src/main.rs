mod config;
mod load;
mod metrics;
mod server;
use config::{BenchResult, Config, Random};
use load::{LoadConfig, measure};
use metrics::median;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn text_command(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command)
        .args(args)
        .current_dir(server::root())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
fn write_report(path: &Path, report: &Value) -> BenchResult<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut json = serde_json::to_string_pretty(report)?;
    json.push('\n');
    // An interrupted write must not leave a report that looks complete.
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    std::io::Write::write_all(&mut file, json.as_bytes())?;
    file.persist(path)?;
    Ok(())
}
fn summary(runs: &[Value]) -> Value {
    let mut result = json!({"runs":runs.len(),"passed":!runs.is_empty() && runs.iter().all(|v| v["passed"] == true)});
    for (input, output) in [
        ("responses_per_second", "median_responses_per_second"),
        ("p50_ms", "median_p50_ms"),
        ("p95_ms", "median_p95_ms"),
        ("p99_ms", "median_p99_ms"),
        ("startup_seconds", "median_startup_seconds"),
        (
            "client_cpu_percent_one_core",
            "median_client_cpu_percent_one_core",
        ),
        (
            "server_cpu_percent_one_core",
            "median_server_cpu_percent_one_core",
        ),
    ] {
        result[output] = json!(median(
            runs.iter().filter_map(|v| v[input].as_f64()).collect()
        ));
    }
    result
}
fn seed(master: u64, case: &str, framework: &str, run: usize, count: Option<usize>) -> u64 {
    let hash = Sha256::digest(format!("{master}:{case}:{framework}:{run}:{count:?}"));
    u64::from_be_bytes(hash[..8].try_into().unwrap())
}

async fn compare(config: &Config) -> BenchResult<Value> {
    let mut schedule = vec![];
    for case in &config.scenarios {
        for framework in &config.frameworks {
            if case.starts_with("di-graph-") && !framework.starts_with("furnace-rs") {
                continue;
            }
            let counts = if case.starts_with("di-graph-") {
                config.dependency_counts.iter().copied().map(Some).collect()
            } else {
                vec![None]
            };
            for count in counts {
                for run in 0..config.runs {
                    schedule.push((case.clone(), framework.clone(), run, count));
                }
            }
        }
    }
    if schedule.is_empty() {
        return Err("no matching framework/scenario trials".into());
    }
    Random(config.seed).shuffle(&mut schedule);
    let mut results: BTreeMap<(String, String, Option<usize>), Value> = BTreeMap::new();
    let mut execution = vec![];
    let mut passed = true;
    for (case, framework, run, count) in schedule {
        eprintln!(
            "{framework}/{case} run {}/{}{}",
            run + 1,
            config.runs,
            count.map(|v| format!(" nodes={v}")).unwrap_or_default()
        );
        let order_seed = seed(config.seed, &case, &framework, run, count);
        execution.push(json!({"scenario":case,"framework":framework,"run":run,"dependency_count":count,"request_order_seed":order_seed}));
        let item = results.entry((case.clone(),framework.clone(),count)).or_insert_with(|| json!({"scenario":case,"framework":framework,"dependency_count":count,"runs":[],"contract_errors":[],"errors":[]}));
        let trial: BenchResult<Value> = async {
            let artifact = server::artifact(&framework, &case, count)?;
            let mut server = server::Server::start(&framework, &case, count).await?;
            let checks = load::contracts(&case, &server.url, count).await?;
            item["contract_errors"]
                .as_array_mut()
                .unwrap()
                .extend(checks.iter().map(|e| json!(e)));
            if !checks.is_empty() {
                return Err(
                    format!("contract checks failed; log: {}", server.log_path.display()).into(),
                );
            }
            if case == "auth" && run == 0 {
                item["body_limit"] = load::body_limit_probe(&server.url).await;
            }
            let (operations, concurrency) = config.workload(&case);
            let load = LoadConfig {
                case: case.clone(),
                url: server.url.clone(),
                operations,
                concurrency,
                seed: order_seed,
                dependency_count: count,
                server_pid: Some(server.pid()),
            };
            let mut warmup = load.clone();
            warmup.operations = operations.min(200.max(concurrency.saturating_mul(10)));
            let warmup = measure(warmup).await?;
            if !warmup.passed {
                return Err(format!(
                    "warmup failed: {:?}; log: {}",
                    warmup.error_examples,
                    server.log_path.display()
                )
                .into());
            }
            let measured = measure(load).await?;
            let success = measured.passed;
            let mut measured = serde_json::to_value(measured)?;
            measured["run_index"] = json!(run);
            measured["startup_seconds"] = json!(server.startup_seconds);
            measured["server_cpu_affinity"] = json!(metrics::process_affinity(server.pid()));
            measured["artifact"] = artifact;
            measured["warmup"] = serde_json::to_value(warmup)?;
            if !success {
                measured["server_log"] = json!(server.log_path);
            }
            server.stop(success).await?;
            Ok(measured)
        }
        .await;
        match trial {
            Ok(measured) => {
                if measured["passed"] != true {
                    passed = false;
                }
                item["runs"].as_array_mut().unwrap().push(measured);
            }
            Err(error) => {
                eprintln!("trial failed: {error}");
                passed = false;
                item["errors"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!(error.to_string()));
            }
        }
    }
    let mut values = vec![];
    for mut item in results.into_values() {
        let runs = item["runs"].as_array().unwrap();
        let complete = runs.len() == config.runs
            && item["errors"].as_array().unwrap().is_empty()
            && runs.iter().all(|v| v["passed"] == true);
        item["summary"] = summary(runs);
        item["passed"] = json!(complete);
        passed &= complete;
        values.push(item);
    }
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(server::root().join("Cargo.toml"))?)?;
    let mut source = Sha256::new();
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "src/main.rs",
        "src/config.rs",
        "src/load.rs",
        "src/metrics.rs",
        "src/server.rs",
    ] {
        source.update(path);
        source.update(std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(path),
        )?);
    }
    Ok(json!({
        "timestamp_utc":text_command("date",&["-u","+%Y-%m-%dT%H:%M:%SZ"]),
        "timestamp_unix_seconds":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        "platform":text_command("uname",&["-srvmo"]),"logical_cpus":std::thread::available_parallelism()?.get(),
        "cpu_model":std::fs::read_to_string("/proc/cpuinfo").ok().and_then(|v| v.lines().find_map(|line| line.strip_prefix("model name").and_then(|v| v.split_once(':')).map(|(_,v)|v.trim().to_owned()))),
        "git_head":text_command("git",&["rev-parse","HEAD"]),
        "git_dirty":text_command("git",&["status","--porcelain"]).is_some_and(|v|!v.is_empty()),
        "rustc":env!("BENCH_RUSTC"),"go":text_command("go",&["version"]),
        "versions":{"furnace-rs":manifest["workspace"]["package"]["version"].as_str(),"furnace-rs-beta":manifest["workspace"]["package"]["version"].as_str(),"furnace-rs-1.0.2":"1.0.2","axum":"0.8.9","fiber":"2.52.15","postgres":std::env::var("BENCH_POSTGRES_VERSION").unwrap_or_else(|_|"not captured".into())},
        "load_generator":{"language":"Rust","http_client":"reqwest 0.13.4","runtime":"Tokio multi-thread","runtime_worker_threads":tokio::runtime::Handle::current().metrics().num_workers(),"client_cpu_affinity":metrics::process_affinity(std::process::id()),"requested_server_cpu_affinity":std::env::var("BENCH_SERVER_CPUS").ok(),"server_environment":{"GOMAXPROCS":std::env::var("GOMAXPROCS").ok(),"TOKIO_WORKER_THREADS":std::env::var("TOKIO_WORKER_THREADS").ok()},"source_sha256":format!("{:x}",source.finalize()),"binary_sha256":server::sha256(&std::env::current_exe()?)?,"http_version":"HTTP/1.1","connection_policy":"one client per worker; full response bodies consumed; persistent connections except connection-churn","retry_policy":"disabled","redirect_policy":"disabled","latency":"send through complete response body; contract parsing included in trial wall time","percentiles":"nearest rank per run; medians of per-run percentiles","cpu_measurement":"Linux /proc/PID/task/*/schedstat summed across live runtime threads; 100% means one fully used core; null when unavailable","load_model":"closed loop; next operation follows previous response; not an open-loop arrival-rate or coordinated-omission-corrected test"},
        "profile":config.profile,"measured_runs":config.runs,"random_seed":config.seed,
        "request_randomization":{"target_order":"SplitMix64 Fisher-Yates randomized fresh process/workload trials","operation_order":"independent operations shuffled per worker; dependent HTTP steps retain order","seed_derivation":"first 8 SHA-256 bytes of master seed, scenario, framework, run, dependency count"},
        "execution_order":execution,"results":values,"passed":passed,
    }))
}

async fn run(config: Config) -> BenchResult<bool> {
    match config.command.as_str() {
        "help" | "--help" | "-h" => {
            println!(
                "furnace-bench <build|compare|stress|measure> [options]\n\n--framework furnace-rs|axum|fiber|furnace-rs-1.0.2|furnace-rs-beta (repeatable)\n--scenario hello|posts|auth|connection-churn|validation|oversized|oversized-reuse|body-limit-boundary|aborted-upload|jwt|di-graph-noop|di-graph-traverse (repeatable; --case is an alias)\n--profile smoke|stress|extended\n--runs N  --seed N  --operations N  --concurrency N\n--dependency-counts 1,16,64,256\n--url http://127.0.0.1:PORT (measure only)\n--output report.json (required except build/help)\n\ncompare defaults: stress, 5 runs, all 3 frameworks, hello/posts/auth.\nstress defaults: smoke, 1 run, FURNACE HTTP cases.\nbuild compiles optimized servers; graph/baseline setup retains its existing build helper."
            );
            Ok(true)
        }
        "build" => {
            server::build(&config.frameworks)?;
            Ok(true)
        }
        "measure" => {
            let case = config.scenarios[0].clone();
            let (operations, concurrency) = config.workload(&case);
            let result = measure(LoadConfig {
                case,
                url: config.url.unwrap(),
                operations,
                concurrency,
                seed: config.seed,
                dependency_count: config.dependency_counts.first().copied(),
                server_pid: None,
            })
            .await?;
            let passed = result.passed;
            write_report(
                config.output.as_ref().unwrap(),
                &serde_json::to_value(result)?,
            )?;
            Ok(passed)
        }
        _ => {
            let report = compare(&config).await?;
            write_report(config.output.as_ref().unwrap(), &report)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(report["passed"] == true)
        }
    }
}
#[tokio::main]
async fn main() {
    let result = match Config::parse() {
        Ok(config) => {
            tokio::select! { result=run(config) => result, _=tokio::signal::ctrl_c() => Err("benchmark interrupted; no successful report produced".into()) }
        }
        Err(error) => Err(error),
    };
    match result {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("furnace-bench: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_trial_groups_cannot_be_successful() {
        assert_eq!(summary(&[])["passed"], false);
    }

    #[test]
    fn summary_uses_medians_of_runs_and_preserves_failure() {
        let runs = vec![
            json!({"responses_per_second":10.0,"p95_ms":1.0,"passed":true}),
            json!({"responses_per_second":100.0,"p95_ms":9.0,"passed":false}),
            json!({"responses_per_second":20.0,"p95_ms":2.0,"passed":true}),
        ];
        let result = summary(&runs);
        assert_eq!(result["median_responses_per_second"], 20.0);
        assert_eq!(result["median_p95_ms"], 2.0);
        assert_eq!(result["passed"], false);
    }

    #[test]
    fn percentile_keeps_slowest_sample_and_empty_samples_are_absent() {
        assert_eq!(metrics::percentile(&[], 99), None);
        assert_eq!(metrics::percentile(&[1.0, 50.0], 50), Some(1.0));
        assert_eq!(metrics::percentile(&[1.0, 50.0], 95), Some(50.0));
        assert_eq!(metrics::percentile(&[1.0, 50.0], 99), Some(50.0));
    }

    #[test]
    fn seeded_shuffle_is_repeatable_and_never_drops_trials() {
        let mut a: Vec<_> = (0..100).collect();
        let mut b = a.clone();
        Random(42).shuffle(&mut a);
        Random(42).shuffle(&mut b);
        assert_eq!(a, b);
        assert_ne!(a, (0..100).collect::<Vec<_>>());
        a.sort_unstable();
        assert_eq!(a, (0..100).collect::<Vec<_>>());
    }
}
