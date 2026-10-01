use mads::prelude::*;

#[derive(Clone)]
struct MissingProvider;

#[mads::burner]
struct NeedsMissing {
    _missing: MissingProvider,
}

#[mads::furnace]
struct InvalidGraphModule;

impl mads::core::Furnace for InvalidGraphModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.provide::<NeedsMissing>()
    }
}


#[mads::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Mads::burn::<InvalidGraphModule>().await
}
