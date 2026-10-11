use furnace_rs::{burner, cauldron, controller, storage, SealRegistration, Sealable};
#[burner] struct Dependency;
#[storage] struct Repository { #[inject] dependency: Dependency }
#[burner] struct Service { #[inject] repository: Repository, legacy: Dependency }
#[controller] struct Controller { #[inject] service: Service }
impl Sealable for Controller { fn seals() -> SealRegistration<Self> { SealRegistration::new() } }
#[cauldron(root, providers=[Dependency, Repository, Service], controllers=[Controller])]
struct App;
fn main() {}
