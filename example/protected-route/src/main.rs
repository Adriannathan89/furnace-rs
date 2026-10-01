mod auth;

use auth::AuthModule;
use mads::prelude::*;

#[furnace]
struct AppModule;

impl mads::core::Furnace for AppModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.import(LoggerModule).import(AuthModule)
    }
}

#[mads::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Mads::burn::<AppModule>().await
}
