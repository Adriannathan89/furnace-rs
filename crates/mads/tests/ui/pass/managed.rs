//! Confirms supported module and managed-provider declarations compile.

#![deny(missing_docs)]

/// Application module used by the compile fixture.
#[mads::furnace]
pub struct AppModule;

impl mads::core::Furnace for AppModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.provide::<UserRepository>().provide::<UserService>().export::<UserRepository>().export::<UserService>()
    }
}


/// Dependency-free repository used by the service.
#[mads::storage]
pub struct UserRepository;

/// Service whose documented public dependency is preserved on its inner value.
#[mads::burner]
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
