use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let proto_dir = manifest_dir.join("protos");

    let protos = [
        proto_dir.join("auth.proto"),
        proto_dir.join("block.proto"),
        proto_dir.join("block_engine.proto"),
        proto_dir.join("bundle.proto"),
        proto_dir.join("packet.proto"),
        proto_dir.join("relayer.proto"),
        proto_dir.join("searcher.proto"),
        proto_dir.join("shared.proto"),
    ];

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile(&protos, &[proto_dir])
        .unwrap_or_else(|e| panic!("protoc failed: {:?}", e));
}