fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    config.file_descriptor_set_path(output.join("near_future_descriptor.bin"));
    config.compile_protos(
        &["../../protocol/near_future/v1/near_future.proto"],
        &["../../protocol"],
    )?;
    println!("cargo:rerun-if-changed=../../protocol/near_future/v1/near_future.proto");
    Ok(())
}
