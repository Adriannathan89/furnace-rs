use ::furnace::{Furnace, Injector, cauldron};

#[derive(Clone)]
struct Plain;
impl Injector for Plain {
    type Dependencies = ();
    async fn inject((): ()) -> ::furnace::core::Result<Self> {
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
    ) -> ::furnace::core::Result<std::sync::Arc<dyn Service>> {
        Ok(std::sync::Arc::new(Self))
    }
}

#[cauldron(providers = [Plain, std::sync::Arc<dyn Service> => Implementation])]
struct AppCauldron;

fn main() {
    let mut builder = Furnace::builder();
    builder.root::<AppCauldron>().unwrap();
    assert!(builder.analyze().is_valid());
}
