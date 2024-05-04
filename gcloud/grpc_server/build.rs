fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tonic_build::configure()

    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile(&["proto/algo_input.proto"], &["proto"])?;

    Ok(())
}
