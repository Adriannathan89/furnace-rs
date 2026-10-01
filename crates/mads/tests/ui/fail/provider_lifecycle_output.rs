struct Resource;

#[mads::element(lifecycle)]
async fn wrong_output() -> Resource {
    Resource
}

#[mads::element(lifecycle)]
async fn generic_resource<T>() -> mads::core::LifecycleResource<T> {
    todo!()
}

fn main() {}
