use furnace_rs::core::{Injector, LifecycleResource, Result};
struct Resource;
impl Injector for Resource {
    type Dependencies = ();
    async fn inject((): ()) -> Result<Self> {
        Ok(Self)
    }
    fn lifecycle(value: Self) -> LifecycleResource<Self> {
        LifecycleResource::new(value)
    }
}
fn main() {
    let _ = Resource::descriptor();
}
