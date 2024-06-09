fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = tonic_build::configure();

    #[cfg(feature = "client")]
    {
        builder = builder.build_client(true);
    }

    #[cfg(feature = "server")]
    {
        builder = builder.build_server(true);
    }

    builder.compile(&["proto/yral_metadata.proto"], &["proto"])?;
    Ok(())
}
