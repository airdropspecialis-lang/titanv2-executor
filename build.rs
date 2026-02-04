fn main() {
    // Rebuild if build script itself changes
    println!("cargo:rerun-if-changed=build.rs");

    // Rebuild if environment affecting gRPC/TLS changes
    println!("cargo:rerun-if-env-changed=GRPC_URL");
    println!("cargo:rerun-if-env-changed=GRPC_TOKEN");
}
