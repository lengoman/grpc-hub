fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("proto_descriptor.bin"))
        .compile_protos(&[
            "proto/hub.proto", 
            "proto/user_service.proto", 
            "proto/order_service.proto",
            "proto/web_content_extract.proto",
            "proto/dividend_service.proto"
        ], &["proto"])?;

    println!("cargo:rerun-if-changed=proto/hub.proto");
    println!("cargo:rerun-if-changed=proto/user_service.proto");
    println!("cargo:rerun-if-changed=proto/order_service.proto");
    println!("cargo:rerun-if-changed=proto/web_content_extract.proto");
    println!("cargo:rerun-if-changed=proto/dividend_service.proto");
    Ok(())
}