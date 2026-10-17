fn main() {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .expect("rustc version");
    assert!(output.status.success());
    println!(
        "cargo:rustc-env=BENCH_RUSTC={}",
        String::from_utf8(output.stdout).unwrap().trim()
    );
    println!("cargo:rerun-if-changed=build.rs");
}
