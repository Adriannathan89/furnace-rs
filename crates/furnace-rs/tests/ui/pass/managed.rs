//! Confirms supported module and managed-provider declarations compile.

#![deny(missing_docs)]

/// Application module used by the compile fixture.
#[furnace_rs::cauldron]
pub struct AppCauldron;

impl furnace_rs::core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.provide::<UserRepository>().provide::<UserService>().export::<UserRepository>().export::<UserService>()
    }
}


/// Dependency-free repository used by the service.
#[furnace_rs::storage]
pub struct UserRepository;

/// Service whose documented public dependency is preserved on its inner value.
#[furnace_rs::burner]
pub struct UserService {
    /// Repository used by service methods.
    pub repository: UserRepository,
}

impl UserService {
    fn repository(&self) -> &UserRepository {
        &self.repository
    }
}

fn main() {
    let _method: fn(&UserService) -> &UserRepository = UserService::repository;
}
