fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_file = "./proto/mls_service.proto";
    
    // Tell Cargo to recompile if the proto file changes
    println!("cargo:rerun-if-changed={}", proto_file);
    println!("cargo:rerun-if-changed=proto");
    
    // Get the output directory from Cargo
    let out_dir = std::env::var("OUT_DIR").unwrap();
    
    // Compile protos using tonic_build with descriptor file generation enabled
    tonic_build::configure()
        .file_descriptor_set_path(format!("{}/mls_descriptor.bin", out_dir))
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)  // Generate all files in the Cargo OUT_DIR
        .compile_protos(&[proto_file], &["proto"])?;
    
    Ok(())
} 