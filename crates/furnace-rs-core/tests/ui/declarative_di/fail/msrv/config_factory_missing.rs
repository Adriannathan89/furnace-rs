use furnace_rs_core::{cauldron, Config, Injector, Result};
#[derive(Clone)] struct Missing;
struct ConfigFactory;
impl Injector<Config> for ConfigFactory {
    type Dependencies = (Missing,);
    async fn inject(_: Self::Dependencies) -> Result<Config> { Ok(Config::empty()) }
}
#[cauldron(root, providers=[Config => ConfigFactory])]
struct App;
fn main() {}
