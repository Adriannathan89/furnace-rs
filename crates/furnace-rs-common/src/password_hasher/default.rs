use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use super::{PasswordHasherResult, PasswordHasherService};

/// Argon2 password hashing backend with configurable algorithm and cost settings.
///
/// Each hash uses a fresh OS-generated salt. Verification uses the settings
/// encoded in the stored PHC string.
#[derive(Clone, Default)]
pub struct DefaultPassowrdHasher {
    instance: Argon2<'static>,
}

impl DefaultPassowrdHasher {
    /// Creates a backend using the supplied Argon2 configuration.
    ///
    /// The instance contains the algorithm, version, and parameters; no separate
    /// parameter copy is needed.
    pub fn new(instance: Argon2<'static>) -> Self {
        Self { instance }
    }
}

impl PasswordHasherService for DefaultPassowrdHasher {
    fn hash_password(&self, password: &str) -> PasswordHasherResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        Ok(self
            .instance
            .hash_password(password.as_bytes(), &salt)?
            .to_string())
    }

    fn validate_password(&self, password: &str, hashed: &str) -> PasswordHasherResult<bool> {
        let parsed_hash = PasswordHash::new(hashed)?;
        Ok(self
            .instance
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}
