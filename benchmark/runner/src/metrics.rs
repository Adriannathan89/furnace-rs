use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Default)]
pub struct Measurements {
    pub latencies: Vec<f64>,
    pub statuses: BTreeMap<u16, usize>,
    pub attempted_requests: usize,
    pub completed_operations: usize,
    pub error_count: usize,
    pub error_examples: Vec<String>,
}
impl Measurements {
    pub fn error(&mut self, message: impl Into<String>) {
        self.error_count += 1;
        if self.error_examples.len() < 10 {
            self.error_examples.push(message.into());
        }
    }
    pub fn merge(&mut self, other: Self) {
        self.latencies.extend(other.latencies);
        for (status, count) in other.statuses {
            *self.statuses.entry(status).or_default() += count;
        }
        self.attempted_requests += other.attempted_requests;
        self.completed_operations += other.completed_operations;
        self.error_count += other.error_count;
        let available = 10 - self.error_examples.len();
        self.error_examples
            .extend(other.error_examples.into_iter().take(available));
    }
}

#[derive(Serialize)]
pub struct Measurement {
    pub case: String,
    pub operations: usize,
    pub completed_operations: usize,
    pub concurrency: usize,
    pub attempted_requests: usize,
    pub http_responses: usize,
    pub status_counts: BTreeMap<u16, usize>,
    pub duration_seconds: f64,
    pub responses_per_second: f64,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub client_cpu_seconds: Option<f64>,
    pub client_cpu_percent_one_core: Option<f64>,
    pub server_cpu_seconds: Option<f64>,
    pub server_cpu_percent_one_core: Option<f64>,
    pub request_order_seed: u64,
    pub request_order_sha256: String,
    pub error_count: usize,
    pub error_examples: Vec<String>,
    pub passed: bool,
}

pub fn percentile(sorted: &[f64], percentage: usize) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    Some(sorted[(sorted.len() * percentage).div_ceil(100) - 1])
}

pub fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}

// Linux reports cumulative scheduled CPU nanoseconds for every live OS thread.
// Snapshot stable runtime threads before/after each trial, outside the wall timer.
pub fn process_cpu_seconds(pid: u32) -> Option<f64> {
    let directory = PathBuf::from(format!("/proc/{pid}/task"));
    let mut nanos = 0_u64;
    let mut threads = 0;
    for entry in std::fs::read_dir(directory).ok()? {
        let data = std::fs::read_to_string(entry.ok()?.path().join("schedstat")).ok()?;
        nanos = nanos.checked_add(data.split_whitespace().next()?.parse::<u64>().ok()?)?;
        threads += 1;
    }
    (threads > 0).then_some(nanos as f64 / 1e9)
}
pub fn cpu_delta(before: Option<f64>, after: Option<f64>) -> Option<f64> {
    let delta = after? - before?;
    (delta >= 0.0).then_some(delta)
}

pub fn process_affinity(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("Cpus_allowed_list:")
                .map(|value| value.trim().to_owned())
        })
}
