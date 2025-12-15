//! Build script for Tycho gRPC - Compiles gRPC protocol buffers.
//!
//! This script uses tonic-build to generate Rust code from the proto definitions.
//! Proto files are in the root `proto/` directory.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the Tycho Bridge v1 proto file
    // Proto path is relative to workspace root (two levels up from this crate)
    tonic_build::configure()
        // Generate server trait for implementing the service
        .build_server(true)
        // Generate client for testing
        .build_client(true)
        // Use protobuf well-known types where applicable
        .protoc_arg("--experimental_allow_proto3_optional")
        // Compile the proto file
        .compile_protos(
            &["../../proto/tycho/bridge/v1/bridge.proto"],
            &["../../proto"],
        )?;

    // Tell Cargo to rerun this build script if the proto file changes
    println!("cargo:rerun-if-changed=../../proto/tycho/bridge/v1/bridge.proto");

    Ok(())
}
