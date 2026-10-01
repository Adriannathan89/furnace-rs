mod post;

use mads::prelude::*;
use mads_persistence::sea_orm::DatabaseModule;
use post::PostModule;

#[furnace]
struct AppModule;

impl mads::core::Furnace for AppModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.import(DatabaseModule).import(PostModule)
    }
}

#[mads::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Mads::burn::<AppModule>().await
}
