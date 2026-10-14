fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = tonic_prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    tonic_prost_build::configure().file_descriptor_set_path(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("service.bin"))
        .compile_with_config(config, &["proto/service.proto"], &["proto"])?;
    Ok(())
}
