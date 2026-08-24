fn main() {
    println!("cargo:rerun-if-changed=proto");
    unsafe {
        std::env::set_var("PROTOC", protobuf_src::protoc());
    }
    let mut build = prost_build::Config::new();
    build
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&["proto/monyacode.proto"], &["proto"])
        .unwrap();
}
