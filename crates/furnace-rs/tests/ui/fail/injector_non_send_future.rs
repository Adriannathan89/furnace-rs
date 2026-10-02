use furnace_rs::Injector;
struct Service;
impl Injector for Service {
    type Dependencies = ();
    async fn inject((): ()) -> furnace_rs::core::Result<Self> {
        let state = std::rc::Rc::new(1);
        std::future::pending::<()>().await;
        let _ = *state;
        Ok(Self)
    }
}
fn main() {}
