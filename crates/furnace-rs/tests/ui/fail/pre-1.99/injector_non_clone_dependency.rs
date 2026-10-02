use furnace_rs::Injector;
struct Dependency;
struct Service;
impl Injector for Service {
    type Dependencies = (Dependency,);
    async fn inject(_: Self::Dependencies) -> furnace_rs::core::Result<Self> {
        Ok(Self)
    }
}
fn main() {}
