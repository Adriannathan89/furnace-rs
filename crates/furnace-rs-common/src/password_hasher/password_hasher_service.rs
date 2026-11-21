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

/// Non-blocking variants for async callers.
///
/// Argon2 is deliberately slow and memory-hard. Calling the synchronous methods
/// from an async handler stalls a runtime worker for the whole computation, so
/// a handful of concurrent logins can freeze every other request on the server.
/// These methods run the same work on Tokio's blocking pool instead.
///
/// At most [`max_concurrent_async_operations`](Self::max_concurrent_async_operations)
/// run at once across the process; further callers wait their turn. The bound
/// keeps a burst of logins from occupying hundreds of blocking threads, each
/// holding a full Argon2 memory allocation. A slot is released only when its
/// work finishes, so dropping the returned future does not free it early.
///
/// Available with the `password-hasher-async` feature, and whenever the
/// `transport-runtime` or `http` feature already links Tokio. The methods must
/// be awaited inside a Tokio runtime.
#[cfg(any(feature = "password-hasher-async", feature = "transport-runtime"))]
impl PasswordHasher {
    /// Largest number of `*_async` operations that run at the same time.
    ///
    /// Equals the parallelism available to the process, since each operation
    /// keeps one core busy.
    pub fn max_concurrent_async_operations() -> usize {
        std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
    }

    /// Hashes a password without blocking the async runtime.
    ///
    /// # Errors
    ///
    /// Propagates the backend's hashing errors unchanged, and returns a
    /// cryptographic error if the runtime shuts down before the work completes.
    ///
    /// # Panics
    ///
    /// Panics outside a Tokio runtime, and resumes a panic raised by the backend.
    pub async fn hash_password_async(&self, password: &str) -> PasswordHasherResult<String> {
        let password = password.to_owned();

        self.run_blocking(move |hasher| hasher.hash_password(&password))
            .await
    }

    /// Checks a password without blocking the async runtime.
    ///
    /// # Errors
    ///
    /// Propagates the backend's validation errors unchanged, and returns a
    /// cryptographic error if the runtime shuts down before the work completes.
    ///
    /// # Panics
    ///
    /// Panics outside a Tokio runtime, and resumes a panic raised by the backend.
    pub async fn validate_password_async(
        &self,
        password: &str,
        hashed: &str,
    ) -> PasswordHasherResult<bool> {
        self.validate_optional_password_async(password, Some(hashed))
            .await
    }

    /// Checks a login attempt for a possibly missing account without blocking
    /// the async runtime.
    ///
    /// See [`validate_optional_password`](Self::validate_optional_password).
    ///
    /// # Errors
    ///
    /// Propagates the backend's errors unchanged, and returns a cryptographic
    /// error if the runtime shuts down before the work completes.
    ///
    /// # Panics
    ///
    /// Panics outside a Tokio runtime, and resumes a panic raised by the backend.
    pub async fn validate_optional_password_async(
        &self,
        password: &str,
        hashed: Option<&str>,
    ) -> PasswordHasherResult<bool> {
        let password = password.to_owned();
        let hashed = hashed.map(str::to_owned);

        self.run_blocking(move |hasher| {
            hasher.validate_optional_password(&password, hashed.as_deref())
        })
        .await
    }

    /// Runs backend work on the blocking pool under the process-wide bound.
    async fn run_blocking<T, F>(&self, work: F) -> PasswordHasherResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Self) -> PasswordHasherResult<T> + Send + 'static,
    {
        static SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();

        let slots = SLOTS.get_or_init(|| {
            Arc::new(tokio::sync::Semaphore::new(
                Self::max_concurrent_async_operations(),
            ))
        });
        // The semaphore is never closed, so acquisition only fails defensively.
        let slot = Arc::clone(slots)
            .acquire_owned()
            .await
            .map_err(|_| super::PasswordHasherError::Crypto)?;
        let hasher = self.clone();
        let task = tokio::task::spawn_blocking(move || {
            // Owned by the blocking task so an abandoned caller cannot free it early.
            let _slot = slot;
            work(&hasher)
        });

        match task.await {
            Ok(result) => result,
            Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
            Err(_) => Err(super::PasswordHasherError::Crypto),
        }
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new(DefaultPasswordHasher::default())
    }
}
