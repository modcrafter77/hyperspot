fn main() {
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile(&["proto/restinvoke.proto"], &["proto"])
        .unwrap();
}

