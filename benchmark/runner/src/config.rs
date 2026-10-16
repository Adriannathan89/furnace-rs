use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub type BenchResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const USERNAME: &str = "demo";
pub const PASSWORD: &str = "correct-horse-battery-staple";
pub const SECRET: &str = "benchmark-only-signing-key-change-me-2026";
pub const FRAMEWORKS: &[&str] = &[
    "furnace-rs",
    "axum",
    "fiber",
    "furnace-rs-1.0.2",
    "furnace-rs-beta",
];
pub const CASES: &[&str] = &[
    "hello",
    "auth",
    "posts",
    "connection-churn",
    "validation",
    "oversized",
    "oversized-reuse",
    "body-limit-boundary",
    "aborted-upload",
    "jwt",
    "di-graph-noop",
    "di-graph-traverse",
];

#[derive(Debug)]
pub struct Config {
    pub command: String,
    pub profile: String,
    pub frameworks: Vec<String>,
    pub scenarios: Vec<String>,
    pub runs: usize,
    pub seed: u64,
    pub operations: Option<usize>,
    pub concurrency: Option<usize>,
    pub dependency_counts: Vec<usize>,
    pub url: Option<String>,
    pub output: Option<PathBuf>,
}

impl Config {
    pub fn parse() -> BenchResult<Self> {
        let mut args = std::env::args().skip(1);
        let command = args.next().unwrap_or_else(|| "help".into());
        if ![
            "compare", "stress", "measure", "build", "help", "--help", "-h",
        ]
        .contains(&command.as_str())
        {
            return Err(format!("unknown command: {command}").into());
        }
        let mut config = Self {
            profile: if command == "stress" {
                "smoke"
            } else {
                "stress"
            }
            .into(),
            runs: if command == "compare" { 5 } else { 1 },
            command,
            frameworks: vec![],
            scenarios: vec![],
            operations: None,
            concurrency: None,
            dependency_counts: vec![],
            url: None,
            output: None,
            seed: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
        };
        while let Some(arg) = args.next() {
            if ["--help", "-h"].contains(&arg.as_str()) {
                config.command = "help".into();
                return Ok(config);
            }
            if ![
                "--profile",
                "--framework",
                "--scenario",
                "--case",
                "--runs",
                "--seed",
                "--operations",
                "--concurrency",
                "--dependency-counts",
                "--url",
                "--output",
            ]
            .contains(&arg.as_str())
            {
                return Err(format!("unknown argument: {arg}").into());
            }
            let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
            match arg.as_str() {
                "--profile" => config.profile = value,
                "--framework" => config.frameworks.push(value),
                "--scenario" | "--case" => config.scenarios.push(value),
                "--runs" => config.runs = positive(&value)?,
                "--operations" => config.operations = Some(positive(&value)?),
                "--concurrency" => config.concurrency = Some(positive(&value)?),
                "--seed" => config.seed = value.parse()?,
                "--dependency-counts" => {
                    config.dependency_counts = value
                        .split(',')
                        .map(positive)
                        .collect::<BenchResult<Vec<_>>>()?;
                    config.dependency_counts.sort_unstable();
                    config.dependency_counts.dedup();
                }
                "--url" => config.url = Some(value),
                "--output" => config.output = Some(value.into()),
                _ => unreachable!(),
            }
        }
        if !["smoke", "stress", "extended"].contains(&config.profile.as_str()) {
            return Err("profile must be smoke, stress, or extended".into());
        }
        for framework in &config.frameworks {
            if !FRAMEWORKS.contains(&framework.as_str()) {
                return Err(format!("unknown framework: {framework}").into());
            }
        }
        for scenario in &config.scenarios {
            if !CASES.contains(&scenario.as_str()) {
                return Err(format!("unknown scenario: {scenario}").into());
            }
        }
        if config.frameworks.is_empty() {
            config.frameworks = if config.command == "stress" {
                vec!["furnace-rs".into()]
            } else {
                vec!["furnace-rs".into(), "axum".into(), "fiber".into()]
            };
        }
        if config.scenarios.is_empty() {
            config.scenarios = if config.command == "stress" {
                vec![
                    "hello",
                    "connection-churn",
                    "validation",
                    "oversized",
                    "oversized-reuse",
                    "body-limit-boundary",
                    "aborted-upload",
                    "jwt",
                    "posts",
                ]
            } else {
                vec!["hello", "posts", "auth"]
            }
            .into_iter()
            .map(str::to_owned)
            .collect();
        }
        if config.command == "measure" && (config.url.is_none() || config.scenarios.len() != 1) {
            return Err("measure requires --url and exactly one --scenario".into());
        }
        if ["measure", "compare", "stress"].contains(&config.command.as_str())
            && config.output.is_none()
        {
            return Err("--output is required".into());
        }
        if config.scenarios.iter().any(|s| s.starts_with("di-graph-"))
            && config.dependency_counts.is_empty()
        {
            return Err(
                "graph workloads require --dependency-counts (and previously built graph binaries)"
                    .into(),
            );
        }
        Ok(config)
    }

    pub fn workload(&self, scenario: &str) -> (usize, usize) {
        let case = if scenario == "auth" {
            "jwt"
        } else if scenario.starts_with("di-graph-") {
            "hello"
        } else {
            scenario
        };
        let (ops, clients) = match (self.profile.as_str(), case) {
            (_, "oversized-reuse") => (8, 4),
            ("smoke", "hello") => (200, 8),
            ("smoke", "connection-churn") => (40, 8),
            ("smoke", "validation" | "jwt") => (90, 8),
            ("smoke", "oversized") => (4, 2),
            ("smoke", "body-limit-boundary") => (12, 4),
            ("smoke", "aborted-upload") => (16, 4),
            ("smoke", "posts") => (20, 4),
            ("stress", "hello") => (12_000, 64),
            ("stress", "connection-churn") => (2_000, 64),
            ("stress", "validation") => (3_000, 32),
            ("stress", "jwt") => (6_000, 64),
            ("stress", "oversized") => (32, 8),
            ("stress", "body-limit-boundary") => (96, 8),
            ("stress", "aborted-upload") => (256, 32),
            ("stress", "posts") => (800, 32),
            ("extended", "hello") => (50_000, 64),
            ("extended", "connection-churn") => (5_000, 64),
            ("extended", "validation") => (10_000, 32),
            ("extended", "jwt") => (25_000, 64),
            ("extended", "oversized") => (64, 8),
            ("extended", "body-limit-boundary") => (192, 16),
            ("extended", "aborted-upload") => (512, 64),
            ("extended", "posts") => (2_000, 32),
            _ => unreachable!("validated case/profile"),
        };
        (
            self.operations.unwrap_or(ops),
            self.concurrency.unwrap_or(clients),
        )
    }
}
fn positive(value: &str) -> BenchResult<usize> {
    let number = value.parse::<usize>()?;
    if number == 0 {
        return Err("counts must be positive".into());
    }
    Ok(number)
}

/// SplitMix64 with rejection sampling: reproducible scheduling without global RNG state.
pub struct Random(pub u64);
impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }
    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            let width = (i + 1) as u64;
            let threshold = width.wrapping_neg() % width;
            let sample = loop {
                let n = self.next();
                if n >= threshold {
                    break n;
                }
            };
            values.swap(i, (sample % width) as usize);
        }
    }
}
