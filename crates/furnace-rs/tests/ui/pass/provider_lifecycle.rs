use furnace_rs::core::LifecycleResource;

struct DirectResource;
struct FallibleResource;

#[furnace_rs::element(lifecycle)]
async fn direct_resource() -> LifecycleResource<DirectResource> {
    LifecycleResource::new(DirectResource)
}

#[furnace_rs::element(lifecycle)]
async fn fallible_resource() -> furnace_rs::core::Result<LifecycleResource<FallibleResource>> {
    Ok(LifecycleResource::new(FallibleResource))
}

fn main() {}
