use furnace_rs::prelude::*;

use super::{model::User, traits::UserRepository};
use std::sync::Arc;
use furnace_rs::prelude::{Config, Injector};


// Repository: deliberately in memory so this project focuses on auth and TPRS.
pub struct DemoUserRepository {
    username: String,
    password: Secret<String>,
}

impl DemoUserRepository {
    pub fn new(username: String, password: Secret<String>) -> Self {
        Self { username, password }
    }
}

#[derive(Configuration)]
#[config(prefix = "demo")]
struct DemoConfig {
    username: String,
    password: Secret<String>,
}

impl Injector<Arc<dyn UserRepository>> for DemoUserRepository {
    type Dependencies = (Config,);
    async fn inject(
        (config,): Self::Dependencies,
    ) -> furnace_rs::core::Result<Arc<dyn UserRepository>> {
        let settings: DemoConfig = config.parse()?;
        Ok(Arc::new(Self::new(settings.username, settings.password)))
    }
}

impl UserRepository for DemoUserRepository {
    fn authenticate(&self, username: &str, password: &str) -> Option<User> {
        (self.username == username && self.password.expose() == password).then(|| User {
            id: 1,
            username: self.username.clone(),
        })
    }

    fn find_by_id(&self, id: u64) -> Option<User> {
        (id == 1).then(|| User {
            id,
            username: self.username.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_credentials_and_unknown_user_id() {
        let repository = DemoUserRepository::new("demo".into(), Secret::new("password123".into()));
        assert!(repository.authenticate("demo", "wrong-password").is_none());
        assert!(repository.authenticate("other", "password123").is_none());
        assert!(repository.find_by_id(2).is_none());
        assert_eq!(
            repository.authenticate("demo", "password123").unwrap().id,
            1
        );
    }
}
