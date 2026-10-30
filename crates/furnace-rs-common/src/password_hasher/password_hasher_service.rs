use std::sync::Arc;

use super::{DefaultPassowrdHasher, PasswordHasherResult};

/// Contract for replaceable synchronous password hashing backends.
///
/// Implement this trait and pass the backend to [`PasswordHasher::new`] to
/// customize password hashing for an application. Password policy and Unicode
/// normalization belong to the caller.
pub trait PasswordHasherService: Send + Sync {
    /// Hashes the supplied password into a string suitable for storage.
    ///
    /// # Errors
    ///
    /// Returns an error if hashing or encoding fails.
    fn hash_password(&self, password: &str) -> PasswordHasherResult<String>;

    /// Checks a password against a stored hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the stored hash cannot be parsed. A backend may also
    /// return errors specific to its verification process.
    fn validate_password(&self, password: &str, hashed: &str) -> PasswordHasherResult<bool>;
}

/// Clonable application password hasher backed by a shared service.
///
/// Defaults to [`DefaultPassowrdHasher`]. Clones share the configured backend.
///
/// ```
/// use furnace_rs_common::PasswordHasher;
///
/// let hasher = PasswordHasher::default();
/// let hashed = hasher.hash_password("my password")?;
/// assert!(hasher.validate_password("my password", &hashed)?);
/// # Ok::<(), furnace_rs_common::PasswordHasherError>(())
/// ```
#[derive(Clone)]
pub struct PasswordHasher {
    inner: Arc<dyn PasswordHasherService>,
}

impl PasswordHasher {
    /// Creates an instance backed by the supplied service.
    pub fn new(inner: impl PasswordHasherService + 'static) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    /// Hashes a password using the configured backend.
    ///
    /// # Errors
    ///
    /// Propagates the backend's hashing errors unchanged. The default backend
    /// can panic if OS randomness is unavailable.
    pub fn hash_password(&self, password: &str) -> PasswordHasherResult<String> {
        self.inner.hash_password(password)
    }

    /// Checks a password using the configured backend.
    ///
    /// # Errors
    ///
    /// Propagates the backend's validation errors unchanged. The default backend
    /// returns parsing errors and converts verification failures to `Ok(false)`.
    pub fn validate_password(&self, password: &str, hashed: &str) -> PasswordHasherResult<bool> {
        self.inner.validate_password(password, hashed)
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new(DefaultPassowrdHasher::default())
    }
}
