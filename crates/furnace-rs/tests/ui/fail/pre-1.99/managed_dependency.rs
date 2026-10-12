//! Managed fields must be cloneable provider handles.
struct NotCloneManaged;
#[furnace_rs::burner]
struct Service {
    #[inject]
    dependency: NotCloneManaged,
}
fn main() {}
