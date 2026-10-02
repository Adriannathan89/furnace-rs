//! Managed fields must be cloneable provider handles.
struct NotCloneManaged;
#[furnace_rs::burner]
struct Service {
    dependency: NotCloneManaged,
}
fn main() {}
