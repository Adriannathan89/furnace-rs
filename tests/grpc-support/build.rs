fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/shared.proto");
    let mut config = tonic_prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    tonic_prost_build::configure()
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("transport.bin"),
        )
        .compile_with_config(config, &["proto/transport.proto"], &["proto"])?;
    Ok(())
}
