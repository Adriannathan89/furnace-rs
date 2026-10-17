use furnace::prelude::*;
use furnace_rs_example_socket_transport::App;
#[furnace::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "jwt")]
    if std::env::args().nth(1).as_deref() == Some("token") {
        let config =
            furnace::socket::common::runtime::load_standard_config(&std::env::current_dir()?)?;
        let jwt = furnace::JwtService::from_config(&config)?;
        println!(
            "{}",
            jwt.sign(
                furnace_rs_example_socket_transport::Claims { id: 42 },
                furnace::JwtSignOptions::access(std::time::Duration::from_secs(300))
            )?
        );
        return Ok(());
    }
    Furnace::burn::<App>().await?;
    Ok(())
}
