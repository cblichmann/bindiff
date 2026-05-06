use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let protoc_path = manifest_dir.join("third_party/protoc/bin/protoc");
    unsafe { env::set_var("PROTOC", protoc_path) };



    let descriptor_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("proto_descriptor.bin");

    // Compile bindiff_config.proto with descriptor set for pbjson
    let mut config = prost_build::Config::new();
    config.file_descriptor_set_path(&descriptor_path);
    config.compile_protos(&["../../bindiff_config.proto"], &["../../"])
        .unwrap_or_else(|e| panic!("Failed to compile bindiff_config.proto: {}", e));

    // Compile binexport2.proto normally
    prost_build::compile_protos(&["../../java/ui/src/main/proto/binexport2.proto"], &["../../java/ui/src/main/proto"])
        .unwrap_or_else(|e| panic!("Failed to compile binexport2.proto: {}", e));

    let descriptor_set = std::fs::read(&descriptor_path)
        .unwrap_or_else(|e| panic!("Failed to read descriptor set: {}", e));

    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)
        .unwrap_or_else(|e| panic!("Failed to register descriptors: {}", e))
        .build(&[".security.bindiff"])
        .unwrap_or_else(|e| panic!("Failed to build pbjson: {}", e));
}
