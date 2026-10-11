use furnace_rs_core::{burner, cauldron, Config, Injector, Result};
struct ConfigFactory;
impl Injector<Config> for ConfigFactory {
    type Dependencies = ();
    async fn inject(_: ()) -> Result<Config> { Ok(Config::empty()) }
}
#[burner] struct Service { config: Config }
#[cauldron(root, providers=[Config => ConfigFactory, Service])]
struct LocalApp;
type Settings = Config;
#[cauldron(providers=[Settings => ConfigFactory], exports=[Settings], global)]
struct ConfigModule;
#[cauldron(root, providers=[Service], imports=[ConfigModule])]
struct GlobalApp;
fn main() {}
