use furnace::prelude::*;

#[derive(Clone)]
struct MissingProvider;

#[furnace::burner]
struct NeedsMissing {
    _missing: MissingProvider,
}

#[furnace::cauldron]
struct InvalidGraphCauldron;

impl furnace::core::Cauldron for InvalidGraphCauldron {
    fn register(self) -> furnace::core::CauldronRegistration<Self> {
        self.provide::<NeedsMissing>()
    }
}


#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<InvalidGraphCauldron>().await
}
