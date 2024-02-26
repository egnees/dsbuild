fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("src/real/network/proto/message_passing.proto")?;
    Ok(())
}
