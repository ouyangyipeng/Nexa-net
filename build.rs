//! Build script for Nexa-net
//!
//! Compiles Protobuf definitions from proto/ directory into Rust types
//! using tonic-build (prost + tonic code generation).

fn main() {
    let proto_dir = std::path::PathBuf::from("proto");

    let proto_files = [
        proto_dir.join("nexa_common.proto"),
        proto_dir.join("identity.proto"),
        proto_dir.join("discovery.proto"),
        proto_dir.join("transport.proto"),
        proto_dir.join("economy.proto"),
    ];

    // Verify all proto files exist before attempting compilation
    for proto_file in &proto_files {
        if !proto_file.exists() {
            panic!(
                "Proto file not found: {}. Ensure proto definitions are in the proto/ directory.",
                proto_file.display()
            );
        }
    }

    tonic_build::configure()
        .btree_map(["."])
        // Generate Rust types under nexa.protocol.* namespaces matching proto packages
        .compile(&proto_files, &[proto_dir])
        .expect("Failed to compile proto definitions. Check proto files for syntax errors.");

    // Tell cargo to rerun build script when proto files change
    for proto_file in &proto_files {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }
}
