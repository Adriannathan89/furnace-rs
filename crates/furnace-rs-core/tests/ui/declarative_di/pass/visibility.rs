use furnace_rs_core::{burner, cauldron, Config, Injector, Result};
use std::sync::Arc;
struct Native;
impl Injector<Arc<String>> for Native {
    type Dependencies = (Config,);
    async fn inject(_: Self::Dependencies) -> Result<Arc<String>> { Ok(Arc::new(String::new())) }
}
#[cauldron(providers = [Arc<String> => Native], exports = [Arc<String>], global)]
struct Infrastructure;
use Infrastructure as Infra;
#[cauldron(imports = [Infra])]
struct Left;
#[cauldron(imports = [Infrastructure])]
struct Right;
#[burner]
struct Consumer { native: Arc<String> }
#[cauldron(root, imports = [Left, Right, Infra], providers = [Consumer])]
struct App;
fn main() {}
