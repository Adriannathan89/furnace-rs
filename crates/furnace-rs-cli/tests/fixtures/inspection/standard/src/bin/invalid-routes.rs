use furnace::prelude::*;

#[furnace::routes]
trait FirstRoutes {
    #[furnace::get("/duplicate")]
    async fn first(&self);
}

#[furnace::routes]
trait SecondRoutes {
    #[furnace::get("/duplicate")]
    async fn second(&self);
}

#[furnace::controller(routes = [FirstRoutes, SecondRoutes])]
struct InvalidRoutesController;

impl FirstRoutes for InvalidRoutesController {
    async fn first(&self) {}
}

impl SecondRoutes for InvalidRoutesController {
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
