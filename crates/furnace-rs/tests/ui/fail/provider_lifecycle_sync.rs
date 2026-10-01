struct Resource;

#[furnace_rs::element(lifecycle)]
fn resource() -> furnace_rs::core::LifecycleResource<Resource> {
    furnace_rs::core::LifecycleResource::new(Resource)
}

fn main() {}
