use furnace::prelude::*;
use furnace_rs_example_grpc_transport::{
    app::{App, Consumer},
    client::{Remote, exercise},
};
#[furnace::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() != Some("client") {
        Furnace::burn::<App>().await?;
        return Ok(());
    }
    let config = ConfigBuilder::new()
        .dotenv(furnace::core::DotenvSource::optional(".env"))
        .source(furnace::core::TomlSource::optional("furnace.toml"))
        .source(furnace::core::EnvSource::new("FURNACE_"))
        .source(furnace::core::MapSource::new(
            "client mode",
            [("grpc.reflection.enabled", "false")],
        ))
        .build()?;
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<Consumer>()?;
    let mut app = builder.build().await?;
    app.start().await?;
    let remote = app.context().resolve::<Remote>()?;
    let result = exercise(&remote).await;
    let shutdown = app.shutdown().await;
    result?;
    shutdown?;
    println!("Unary, server stream, client stream, and bidirectional calls passed.");
    Ok(())
}
