use furnace::prelude::*;

#[derive(Clone)]
struct Marker;

#[furnace::element]
fn marker_provider() -> Marker {
    if let Ok(marker) = std::env::var("FURNACE_TEST_CONSTRUCTION_MARKER") {
        let _ = std::fs::write(marker, "constructed");
    }
    Marker
}

#[furnace::controller]
struct UserController {
    _marker: Marker,
}

impl ::furnace::Sealable for UserController {
    fn seals() -> ::furnace::SealRegistration<Self> {
        ::furnace::SealRegistration::new()
    }
}

#[furnace::controller]
impl UserController {
    #[furnace::get("/users/:id")]
    async fn get_user(&self) -> &'static str {
        "user"
    }
    #[furnace::post("/users")]
    async fn create_user(&self) -> &'static str {
        "created"
    }
}

#[furnace::cauldron]
struct AppCauldron;

impl furnace::core::Cauldron for AppCauldron {
    fn register(self) -> furnace::core::CauldronRegistration<Self> {
        self.provide::<Marker>().controller::<UserController>()
    }
}

#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
