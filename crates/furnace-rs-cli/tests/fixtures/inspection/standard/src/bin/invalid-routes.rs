use furnace::prelude::*;

#[furnace::controller]
struct InvalidRoutesController;

impl ::furnace::Sealable for InvalidRoutesController {
    fn seals() -> ::furnace::SealRegistration<Self> {
        ::furnace::SealRegistration::new()
    }
}

#[furnace::controller]
impl InvalidRoutesController {
    #[furnace::get("/duplicate")]
    async fn first(&self) {}
    #[furnace::get("/duplicate")]
    async fn second(&self) {}
}

#[furnace::cauldron(controllers = [InvalidRoutesController])]
struct InvalidRoutesCauldron;

#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<InvalidRoutesCauldron>().await
}
