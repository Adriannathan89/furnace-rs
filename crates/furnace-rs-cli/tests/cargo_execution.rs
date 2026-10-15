//! Real Cargo fixtures for invocation configuration and development graph changes.

use std::{fs, path::Path, process::Command};

#[cfg(unix)]
use std::{
    process::{Child, Stdio},
    thread,
    time::{Duration, Instant},
};

use assert_cmd::cargo::CommandCargoExt;

#[test]
fn run_retains_member_local_cargo_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=['app']\nresolver='3'\n",
    )
    .unwrap();
    package(
        &root.join("app"),
        "app",
        "#[cfg(not(member_config))] compile_error!(\"member rustflags were lost\"); fn main() { println!(\"{}\", env!(\"FURNACE_MEMBER_CONFIG\")); }",
        "",
    );
    fs::create_dir(root.join("app/.cargo")).unwrap();
    fs::write(root.join("app/.cargo/config.toml"), "[env]\nFURNACE_MEMBER_CONFIG='configured'\n[build]\nrustflags=['--cfg', 'member_config']\n").unwrap();
    let output = Command::cargo_bin("furnace")
        .unwrap()
        .current_dir(root.join("app"))
        .arg("run")
        .env_remove("FURNACE_MEMBER_CONFIG")
        .env("CARGO_TARGET_DIR", root.join("artifacts"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "configured");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn run_supplies_the_prefer_dynamic_toolchain_library_path() {
    let directory = tempfile::tempdir().unwrap();
    package(
        directory.path(),
        "dynamic-app",
        "fn main() { println!(\"DYNAMIC_OK\"); }",
        "[workspace]\n",
    );
    let output = Command::cargo_bin("furnace")
        .unwrap()
        .current_dir(directory.path())
        .arg("run")
        .env("RUSTFLAGS", "-C prefer-dynamic")
        .env("CARGO_TARGET_DIR", directory.path().join("artifacts"))
        // cargo test itself adds toolchain libraries; exercise a clean CLI environment.
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_FALLBACK_LIBRARY_PATH")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "DYNAMIC_OK");
}

#[cfg(target_os = "linux")]
#[test]
fn run_retains_build_script_library_directories_and_existing_search_paths() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    package(
        root,
        "native-app",
        r#"
        #[link(name = "fixture")]
        unsafe extern "C" { fn fixture_value() -> u32; }
        fn main() {
            println!("NATIVE_OK={}", unsafe { fixture_value() });
            println!("{}", std::env::var("LD_LIBRARY_PATH").unwrap());
        }
    "#,
        "[workspace]\n",
    );
    fs::write(
        root.join("native.rs"),
        "#[unsafe(no_mangle)] pub extern \"C\" fn fixture_value() -> u32 { 42 }",
    )
    .unwrap();
    fs::write(
        root.join("build.rs"),
        r#"
        fn main() {
            let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
            assert!(std::process::Command::new(std::env::var_os("RUSTC").unwrap())
                .args(["--crate-type=cdylib", "native.rs", "-o"])
                .arg(output.join("libfixture.so")).status().unwrap().success());
            println!("cargo:rustc-link-search=native={}", output.display());
            println!("cargo:rustc-link-lib=dylib=fixture");
            println!("cargo:rerun-if-changed=native.rs");
        }
    "#,
    )
    .unwrap();
    let output = Command::cargo_bin("furnace")
        .unwrap()
        .current_dir(root)
        .arg("run")
        .env("LD_LIBRARY_PATH", "/tmp/furnace-existing-library-path")
        .env("CARGO_TARGET_DIR", root.join("artifacts"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("NATIVE_OK=42"));
    assert!(stdout.contains("/tmp/furnace-existing-library-path"));
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn inspection_launches_a_prefer_dynamic_protocol_child() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    // A dependency stub supplies the version selection contract; the binary implements
    // the real private wire protocol without rebuilding the full framework dynamically.
    fs::create_dir_all(root.join("furnace/src")).unwrap();
    fs::write(
        root.join("furnace/Cargo.toml"),
        format!(
            "[package]\nname='furnace-rs'\nversion='{}'\nedition='2024'\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    fs::write(root.join("furnace/src/lib.rs"), "").unwrap();
    package(
        root,
        "dynamic-inspection",
        r##"
        fn main() {
            let token = std::env::var("FURNACE_INTERNAL_INSPECTION_TOKEN").unwrap();
            let version = std::env::var("FURNACE_INTERNAL_INSPECTION_VERSION").unwrap();
            let ack = format!("{{\"protocol_version\":{version},\"token\":\"{token}\"}}");
            publish("FURNACE_INTERNAL_INSPECTION_ACK", &ack);
            let report = r#"{"kind":"routes","graph":{"root_cauldron":null,"cauldrons":[],"imports":[],"providers":[],"dependencies":[],"construction_order":null,"auto_configurations":[]},"routes":[],"grpc_services":[],"checks":[],"diagnostics":[],"failed":false}"#;
            let response = format!("{{\"protocol_version\":{version},\"token\":\"{token}\",\"report\":{report}}}");
            publish("FURNACE_INTERNAL_INSPECTION_RESPONSE", &response);
        }
        fn publish(variable: &str, text: &str) {
            let path = std::path::PathBuf::from(std::env::var_os(variable).unwrap());
            let temporary = path.with_extension("tmp");
            std::fs::write(&temporary, text).unwrap();
            std::fs::rename(temporary, path).unwrap();
        }
    "##,
        "[workspace]\n[dependencies]\nfurnace-rs={path='furnace'}\n",
    );
    let output = Command::cargo_bin("furnace")
        .unwrap()
        .current_dir(root)
        .arg("routes")
        .env("RUSTFLAGS", "-C prefer-dynamic")
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_FALLBACK_LIBRARY_PATH")
        .env("CARGO_TARGET_DIR", root.join("artifacts"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn dev_rebuilds_build_inputs_and_refreshes_added_and_removed_path_dependencies() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let app = root.join("app");
    let dependency = root.join("dependency");
    fs::create_dir_all(dependency.join("src")).unwrap();
    fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname='dependency'\nversion='0.1.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    fs::write(
        dependency.join("src/lib.rs"),
        "pub const VALUE: &str = \"dep-v1\";",
    )
    .unwrap();
    let source = r#"
fn main() {
    use std::io::Write;
    let value = include_str!(concat!(env!("OUT_DIR"), "/schema"));
    let mut log = std::fs::OpenOptions::new().create(true).append(true).open(std::env::var_os("START_LOG").unwrap()).unwrap();
    writeln!(log, "{}|{}", std::process::id(), value).unwrap();
    let shutdown = std::path::PathBuf::from(std::env::var_os("FURNACE_INTERNAL_DEV_SHUTDOWN_PATH").unwrap());
    while !shutdown.exists() { std::thread::sleep(std::time::Duration::from_millis(10)); }
}
"#;
    package(&app, "watch-app", source, "[workspace]\n");
    fs::write(app.join("build.rs"), r#"fn main() {
        println!("cargo:rerun-if-changed=service.proto");
        println!("cargo:rerun-if-changed=data/custom.input");
        std::fs::write(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("schema"), format!("{}:{}", std::fs::read_to_string("service.proto").unwrap(), std::fs::read_to_string("data/custom.input").unwrap())).unwrap();
    }"#).unwrap();
    fs::create_dir(app.join("data")).unwrap();
    fs::write(app.join("service.proto"), "schema-v1").unwrap();
    fs::write(app.join("data/custom.input"), "input-v1").unwrap();
    let log = root.join("starts.log");
    let output_path = root.join("dev.log");
    let output = fs::File::create(&output_path).unwrap();
    let compiler = slow_compiler(root);
    let child = Command::cargo_bin("furnace")
        .unwrap()
        .current_dir(&app)
        .args(["dev", "--bin", "watch-app"])
        .env("RUSTC", &compiler)
        .env("FURNACE_QUERY_LOG", root.join("queries.log"))
        .env("START_LOG", &log)
        .env("CARGO_TARGET_DIR", root.join("artifacts"))
        .env("RUSTFLAGS", "-C prefer-dynamic")
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_FALLBACK_LIBRARY_PATH")
        .stdin(Stdio::null())
        .stdout(output.try_clone().unwrap())
        .stderr(output)
        .spawn()
        .unwrap();
    let mut dev = DevChild {
        child,
        log: log.clone(),
        metadata_pid: None,
    };
    wait_for(&log, "schema-v1:input-v1", &output_path);
    fs::write(app.join("service.proto"), "schema-v2").unwrap();
    wait_for(&log, "schema-v2:input-v1", &output_path);
    fs::write(app.join("data/custom.input"), "input-v2").unwrap();
    wait_for(&log, "schema-v2:input-v2", &output_path);
    assert_eq!(
        fs::read_to_string(&log).unwrap().lines().count(),
        3,
        "one restart per input edit"
    );

    let starts = fs::read_to_string(&log).unwrap();
    fs::write(app.join("runtime.log"), "application output").unwrap();
    fs::write(app.join("cache.db"), "application cache").unwrap();
    thread::sleep(Duration::from_millis(600));
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        starts,
        "runtime outputs must not trigger a rebuild"
    );

    // Dependency is outside the application root and was absent from initial metadata.
    let manifest = fs::read_to_string(app.join("Cargo.toml")).unwrap();
    fs::write(
        app.join("src/main.rs"),
        source.replace(
            "let value = include_str!(concat!(env!(\"OUT_DIR\"), \"/schema\"));",
            "let value = dependency::VALUE;",
        ),
    )
    .unwrap();
    fs::write(
        app.join("Cargo.toml"),
        format!("{manifest}\n[dependencies]\ndependency={{path='../dependency'}}\n"),
    )
    .unwrap();
    wait_for(&log, "dep-v1", &output_path);
    thread::sleep(Duration::from_millis(800));
    fs::write(
        dependency.join("src/lib.rs"),
        "pub const VALUE: &str = \"dep-v2\";",
    )
    .unwrap();
    wait_for(&log, "dep-v2", &output_path);
    let before_removal = fs::read_to_string(&log).unwrap().lines().count();
    fs::write(app.join("src/main.rs"), source).unwrap();
    fs::write(app.join("Cargo.toml"), &manifest).unwrap();
    wait_for_count(&log, before_removal + 1, &output_path);
    thread::sleep(Duration::from_millis(1000));
    let starts = fs::read_to_string(&log).unwrap();
    fs::write(
        dependency.join("src/lib.rs"),
        "pub const VALUE: &str = \"dep-v3\";",
    )
    .unwrap();
    thread::sleep(Duration::from_millis(600));
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        starts,
        "removed dependency must no longer trigger restarts"
    );

    fs::write(app.join("Cargo.toml"), "this is not valid TOML").unwrap();
    wait_until(&output_path, || {
        fs::read_to_string(&output_path)
            .unwrap()
            .contains("metadata reload failed; continuing to watch")
    });
    assert!(
        dev.child.try_wait().unwrap().is_none(),
        "invalid manifests must remain recoverable"
    );
    let before_recovery = fs::read_to_string(&log).unwrap().lines().count();
    fs::write(app.join("Cargo.toml"), &manifest).unwrap();
    wait_for_count(&log, before_recovery + 1, &output_path);

    // The explicit original selection becomes invalid when the manifest renames the binary.
    fs::write(
        app.join("Cargo.toml"),
        format!("{manifest}\n[[bin]]\nname='replacement'\npath='src/main.rs'\n"),
    )
    .unwrap();
    wait_until(&output_path, || dev.child.try_wait().unwrap().is_some());
    let output = fs::read_to_string(&output_path).unwrap();
    assert!(output.contains("FURNACE200"), "{output}");
    assert!(
        output.contains("no binary target named `watch-app`"),
        "{output}"
    );
}

fn package(root: &Path, name: &str, source: &str, extra_manifest: &str) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n{extra_manifest}"),
    )
    .unwrap();
    fs::write(root.join("src/main.rs"), source).unwrap();
}

#[cfg(unix)]
struct DevChild {
    child: Child,
    log: std::path::PathBuf,
    metadata_pid: Option<std::path::PathBuf>,
}
#[cfg(unix)]
impl Drop for DevChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(path) = &self.metadata_pid
            && let Ok(pid) = fs::read_to_string(path)
        {
            let _ = Command::new("kill")
                .args(["-KILL", pid.trim()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        for line in fs::read_to_string(&self.log).unwrap_or_default().lines() {
            if let Some((pid, _)) = line.split_once('|') {
                let _ = Command::new("kill")
                    .args(["-KILL", pid])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
    }
}

#[cfg(unix)]
fn wait_for(log: &Path, expected: &str, output: &Path) {
    wait_until(output, || {
        fs::read_to_string(log)
            .unwrap_or_default()
            .lines()
            .any(|line| line.ends_with(&format!("|{expected}")))
    });
}
#[cfg(unix)]
fn wait_for_count(log: &Path, count: usize, output: &Path) {
    wait_until(output, || {
        fs::read_to_string(log).unwrap_or_default().lines().count() >= count
    });
}
#[cfg(unix)]
fn wait_until(output: &Path, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !ready() {
        assert!(
            Instant::now() < deadline,
            "timed out: {}",
            fs::read_to_string(output).unwrap_or_default()
        );
        thread::sleep(Duration::from_millis(25));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn runtime_query_honors_the_cargo_configured_compiler_and_sysroot() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    package(
        root,
        "configured-runtime",
        "fn main() { println!(\"{}\", std::env::var(\"LD_LIBRARY_PATH\").unwrap()); }",
        "[workspace]\n",
    );
    let sysroot = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(sysroot.status.success());
    let sysroot = std::path::PathBuf::from(String::from_utf8(sysroot.stdout).unwrap().trim());
    let selected_sysroot = root.join("selected-sysroot");
    fs::create_dir(&selected_sysroot).unwrap();
    symlink(sysroot.join("lib"), selected_sysroot.join("lib")).unwrap();
    let compiler = root.join("selected-rustc");
    fs::write(&compiler, "#!/bin/sh\nfor arg in \"$@\"; do\nif [ \"$arg\" = target-libdir ]; then echo QUERY >> \"$FURNACE_QUERY_LOG\"; sleep 0.25; fi\ndone\nexec rustc \"$@\"\n").unwrap();
    fs::set_permissions(&compiler, fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir(root.join(".cargo")).unwrap();
    fs::write(
        root.join(".cargo/config.toml"),
        format!(
            "[build]\nrustc={:?}\nrustflags=['-C', 'prefer-dynamic', '--sysroot', {:?}]\n",
            compiler.to_str().unwrap(),
            selected_sysroot.to_str().unwrap()
        ),
    )
    .unwrap();
    let query_log = root.join("query.log");
    let output = Command::cargo_bin("furnace")
        .unwrap()
        .current_dir(root)
        .arg("run")
        .env("FURNACE_QUERY_LOG", &query_log)
        .env_remove("RUSTC")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("LD_LIBRARY_PATH")
        .env("CARGO_TARGET_DIR", root.join("artifacts"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        query_log.is_file(),
        "runtime discovery must run through Cargo's configured compiler"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains(selected_sysroot.to_str().unwrap()));
}

#[cfg(unix)]
fn slow_compiler(root: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let compiler = root.join("slow-rustc");
    fs::write(&compiler, "#!/bin/sh\nfor arg in \"$@\"; do\nif [ \"$arg\" = target-libdir ]; then echo QUERY >> \"$FURNACE_QUERY_LOG\"; sleep 0.25; fi\ndone\nexec rustc \"$@\"\n").unwrap();
    fs::set_permissions(&compiler, fs::Permissions::from_mode(0o700)).unwrap();
    compiler
}

#[cfg(unix)]
#[test]
fn dev_shutdown_cancels_a_pending_metadata_reload() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let app = root.join("app");
    let source = r#"
        fn main() {
            std::fs::write(std::env::var_os("START_LOG").unwrap(), format!("{}|started\n", std::process::id())).unwrap();
            let shutdown = std::path::PathBuf::from(std::env::var_os("FURNACE_INTERNAL_DEV_SHUTDOWN_PATH").unwrap());
            while !shutdown.exists() { std::thread::sleep(std::time::Duration::from_millis(10)); }
        }
    "#;
    package(&app, "metadata-app", source, "[workspace]\n");
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let cargo = bin.join("cargo");
    fs::write(&cargo, "#!/bin/sh\nif [ \"$1\" = metadata ] && [ -e \"$FURNACE_METADATA_STALL\" ]; then echo $$ > \"$FURNACE_METADATA_PID\"; exec sleep 30; fi\nexec \"$FURNACE_REAL_CARGO\" \"$@\"\n").unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o700)).unwrap();
    let mut path = vec![bin];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let log = root.join("start.log");
    let metadata_pid = root.join("metadata.pid");
    let stall = root.join("stall");
    let output_path = root.join("dev.log");
    let output = fs::File::create(&output_path).unwrap();
    let child = Command::cargo_bin("furnace")
        .unwrap()
        .current_dir(&app)
        .arg("dev")
        .env("CARGO", &cargo)
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("FURNACE_REAL_CARGO", std::env::var_os("CARGO").unwrap())
        .env("FURNACE_METADATA_PID", &metadata_pid)
        .env("FURNACE_METADATA_STALL", &stall)
        .env("START_LOG", &log)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .stdin(Stdio::null())
        .stdout(output.try_clone().unwrap())
        .stderr(output)
        .spawn()
        .unwrap();
    let mut dev = DevChild {
        child,
        log: log.clone(),
        metadata_pid: Some(metadata_pid.clone()),
    };
    wait_for(&log, "started", &output_path);
    fs::write(&stall, "").unwrap();
    let manifest = fs::read_to_string(app.join("Cargo.toml")).unwrap();
    fs::write(
        app.join("Cargo.toml"),
        format!("{manifest}\n# reload metadata\n"),
    )
    .unwrap();
    wait_until(&output_path, || metadata_pid.is_file());
    assert!(
        Command::new("kill")
            .args(["-INT", &dev.child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Some(status) = dev.child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(
            Instant::now() < deadline,
            "shutdown waited on stalled metadata: {}",
            fs::read_to_string(&output_path).unwrap()
        );
        thread::sleep(Duration::from_millis(20));
    }
    let pid = fs::read_to_string(&metadata_pid).unwrap();
    wait_until(&output_path, || {
        !Command::new("kill")
            .args(["-0", pid.trim()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    });
}
