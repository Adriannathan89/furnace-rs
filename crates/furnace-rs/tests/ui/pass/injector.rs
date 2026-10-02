//! Facade and prelude expose explicit construction without factory attributes.
use furnace_rs::prelude::*;
use std::sync::Arc;
trait Service: Send + Sync {}
struct Implementation;
impl Service for Implementation {}
impl Injector<Arc<dyn Service>> for Implementation {
    type Dependencies = ();
    async fn inject((): ()) -> furnace_rs::core::Result<Arc<dyn Service>> {
        Ok(Arc::new(Self))
    }
}
#[burner]
struct Managed;
struct Plain;
impl Injector for Plain {
    type Dependencies = (Managed,);
    async fn inject((_,): Self::Dependencies) -> furnace_rs::core::Result<Self> {
        Ok(Self)
    }
    fn lifecycle(value: Self) -> furnace_rs::core::LifecycleResource<Self> {
        furnace_rs::core::LifecycleResource::new(value)
    }
}
#[cauldron]
struct App;
impl Cauldron for App {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<Managed>()
            .provide::<Plain>()
            .provide_with::<Arc<dyn Service>, Implementation>()
    }
}
fn main() {
    fn dependencies<T: InjectionDependencies>() {}
    dependencies::<()>();
    let _ = <Managed as Injector>::descriptor();
}
