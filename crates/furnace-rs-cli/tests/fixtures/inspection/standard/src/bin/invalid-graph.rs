use furnace::prelude::*;

#[derive(Clone)]
struct MissingProvider;

#[furnace::burner]
struct NeedsMissing {
    #[inject]
    _missing: MissingProvider,
}

#[furnace::cauldron(providers = [NeedsMissing])]
struct InvalidGraphCauldron;

#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<InvalidGraphCauldron>().await
}
