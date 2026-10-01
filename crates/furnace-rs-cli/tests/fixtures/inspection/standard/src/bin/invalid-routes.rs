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

#[furnace::cauldron]
struct InvalidRoutesCauldron;

impl furnace::core::Cauldron for InvalidRoutesCauldron {
    fn register(self) -> furnace::core::CauldronRegistration<Self> {
        self.controller::<InvalidRoutesController>()
    }
}

#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<InvalidRoutesCauldron>().await
}
