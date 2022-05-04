fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/helloworld.proto");
    tonic_build::compile_protos("proto/helloworld.proto")?;
    Ok(())
}
