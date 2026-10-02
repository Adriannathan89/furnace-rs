use ::framework::{Cauldron, CauldronRegistration, Furnace, Injector, cauldron};

#[derive(Clone)]
struct Plain;
impl Injector for Plain {
    type Dependencies = ();
    async fn inject((): ()) -> ::framework::core::Result<Self> {
        Ok(Self)
    }
}
trait Service: Send + Sync {}
struct Implementation;
impl Service for Implementation {}
impl Injector<std::sync::Arc<dyn Service>> for Implementation {
    type Dependencies = (Plain,);
    async fn inject(
        (_,): Self::Dependencies,
    ) -> ::framework::core::Result<std::sync::Arc<dyn Service>> {
        Ok(std::sync::Arc::new(Self))
    }
}

#[cauldron]
struct AppCauldron;
impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<Plain>()
            .provide_with::<std::sync::Arc<dyn Service>, Implementation>()
    }
}
fn main() {
    let mut builder = Furnace::builder();
    builder.root::<AppCauldron>().unwrap();
    assert!(builder.analyze().is_valid());
}
