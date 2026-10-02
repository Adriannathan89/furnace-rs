use std::sync::Arc;

use furnace_rs::prelude::*;

use super::{
    repository::DemoUserRepository,
    service::AuthServiceImpl,
    traits::{AuthService, UserRepository},
};

#[derive(Configuration)]
#[config(prefix = "demo")]
struct DemoConfig {
    username: String,
    password: Secret<String>,
}

// Each implementer constructs the trait output selected by its cauldron.
impl Injector<Arc<dyn UserRepository>> for DemoUserRepository {
    type Dependencies = (Config,);
    async fn inject(
        (config,): Self::Dependencies,
    ) -> furnace_rs::core::Result<Arc<dyn UserRepository>> {
        let settings: DemoConfig = config.parse()?;
        Ok(Arc::new(Self::new(settings.username, settings.password)))
    }
}

impl Injector<Arc<dyn AuthService>> for AuthServiceImpl {
    type Dependencies = (AuthServiceImpl,);
    async fn inject(
        (service,): <Self as Injector<Arc<dyn AuthService>>>::Dependencies,
    ) -> furnace_rs::core::Result<Arc<dyn AuthService>> {
        Ok(Arc::new(service))
    }
}
