struct Resource;

#[furnace_rs::element(lifecycle)]
async fn wrong_output() -> Resource {
    Resource
}

#[furnace_rs::element(lifecycle)]
async fn generic_resource<T>() -> furnace_rs::core::LifecycleResource<T> {
    todo!()
}

fn main() {}
