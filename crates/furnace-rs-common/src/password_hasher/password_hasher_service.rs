use std::sync::{Arc, OnceLock};

use super::{DefaultPasswordHasher, PasswordHasherResult};

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
/// Defaults to [`DefaultPasswordHasher`]. Clones share the configured backend.
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
    shared: Arc<Shared>,
}

/// State shared by every clone of one [`PasswordHasher`].
struct Shared {
    backend: Box<dyn PasswordHasherService>,
    /// Lazily created hash verified in place of a missing stored hash.
    decoy: OnceLock<String>,
}

/// Plaintext behind the decoy hash. It is not a credential: decoy verification
/// always reports a mismatch, whatever the backend answers.
const DECOY_PASSWORD: &str = "furnace-rs decoy password";

impl PasswordHasher {
    /// Creates an instance backed by the supplied service.
    pub fn new(inner: impl PasswordHasherService + 'static) -> Self {
        Self {
            shared: Arc::new(Shared {
                backend: Box::new(inner),
                decoy: OnceLock::new(),
            }),
        }
    }

    /// Hashes a password using the configured backend.
    ///
    /// # Errors
    ///
    /// Propagates the backend's hashing errors unchanged. The default backend
    /// can panic if OS randomness is unavailable.
    pub fn hash_password(&self, password: &str) -> PasswordHasherResult<String> {
        self.shared.backend.hash_password(password)
    }

    /// Checks a password using the configured backend.
    ///
    /// # Errors
    ///
    /// Propagates the backend's validation errors unchanged. The default backend
    /// returns parsing errors, rejects stored hashes whose costs exceed its
    /// verification limits, and converts verification failures to `Ok(false)`.
    pub fn validate_password(&self, password: &str, hashed: &str) -> PasswordHasherResult<bool> {
        self.shared.backend.validate_password(password, hashed)
    }

    /// Checks a login attempt whose account, and therefore stored hash, may not exist.
    ///
    /// Pass the result of the account lookup directly. With `Some(hashed)` this
    /// is [`validate_password`](Self::validate_password). With `None` it still
    /// spends one verification, against a decoy hash created by this backend,
    /// and returns `Ok(false)`. Returning early for unknown accounts instead
    /// lets response time reveal which usernames exist.
    ///
    /// ```
    /// use furnace_rs_common::{PasswordHasher, PasswordHasherResult};
    ///
    /// fn login(
    ///     hasher: &PasswordHasher,
    ///     stored_hash: Option<&str>,
    ///     password: &str,
    /// ) -> PasswordHasherResult<bool> {
    ///     hasher.validate_optional_password(password, stored_hash)
    /// }
    ///
    /// let hasher = PasswordHasher::default();
    /// let stored = hasher.hash_password("secret")?;
    /// assert!(login(&hasher, Some(&stored), "secret")?);
    /// assert!(!login(&hasher, None, "secret")?);
    /// # Ok::<(), furnace_rs_common::PasswordHasherError>(())
    /// ```
    ///
    /// The decoy is hashed on first use and shared by all clones, so the first
    /// unknown-account attempt costs one extra hash.
    ///
    /// # Errors
    ///
    /// Propagates the backend's validation errors, and its hashing errors when
    /// the decoy cannot be created.
    pub fn validate_optional_password(
        &self,
        password: &str,
        hashed: Option<&str>,
    ) -> PasswordHasherResult<bool> {
        if let Some(hashed) = hashed {
            return self.validate_password(password, hashed);
        }

        let decoy = match self.shared.decoy.get() {
            Some(decoy) => decoy,
            None => {
                let created = self.hash_password(DECOY_PASSWORD)?;
                self.shared.decoy.get_or_init(|| created)
            }
        };
        self.validate_password(password, decoy)?;

        Ok(false)
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new(DefaultPasswordHasher::default())
    }
}
