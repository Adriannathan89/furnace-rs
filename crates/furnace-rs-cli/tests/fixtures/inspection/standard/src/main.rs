use furnace::prelude::*;

#[derive(Clone)]
struct Marker;

fn marker_provider() -> Marker {
    if let Ok(marker) = std::env::var("FURNACE_TEST_CONSTRUCTION_MARKER") {
        let _ = std::fs::write(marker, "constructed");
    }
    Marker
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct MarkerProviderInjector;
impl furnace::core::Injector<Marker> for MarkerProviderInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace::core::Result<Marker> {
        Ok(marker_provider())
    }
    fn descriptor() -> &'static furnace::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_MARKER_PROVIDER
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_MARKER_PROVIDER: furnace::core::ProviderDescriptor =
    furnace::core::__private::InjectorMetadata::<Marker, MarkerProviderInjector>::DESCRIPTOR
        .with_authored_type_name("Marker")
        .with_namespace(module_path!())
        .with_visibility(furnace::core::ProviderVisibility::Private)
        .with_location(furnace::core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_MARKER_PROVIDER }

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
        self.provide_with::<Marker, MarkerProviderInjector>()
            .controller::<UserController>()
    }
}

#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
