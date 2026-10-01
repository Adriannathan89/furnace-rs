struct Resource;
struct CustomError;

#[furnace_rs::element(lifecycle)]
async fn resource() -> std::result::Result<furnace_rs::core::LifecycleResource<Resource>, CustomError> {
    Ok(furnace_rs::core::LifecycleResource::new(Resource))
}

fn main() {}
