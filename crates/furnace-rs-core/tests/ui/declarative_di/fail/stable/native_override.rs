use furnace_rs_core::{cauldron, Injector, Result, Furnace};
use std::sync::Arc;
#[derive(Clone)] struct Missing;
struct Native;
impl Injector<Arc<String>> for Native { type Dependencies=(Missing,); async fn inject(_: Self::Dependencies) -> Result<Arc<String>> { Ok(Arc::new(String::new())) } }
#[cauldron(root, providers=[Arc<String> => Native])] struct App;
fn main() { let mut builder = Furnace::builder(); let _ = builder.provide(Arc::new(String::new())); let _ = builder.root::<App>(); }
