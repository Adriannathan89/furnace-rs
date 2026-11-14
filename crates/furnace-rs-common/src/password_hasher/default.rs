use argon2::{
    Argon2, Params,
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString, errors::InvalidValue,
        rand_core::OsRng,
    },
};

use super::{PasswordHasherError, PasswordHasherResult, PasswordHasherService};

/// Argon2 password hashing backend with configurable algorithm and cost settings.
///
/// Each hash uses a fresh OS-generated salt. Verification uses the settings
/// encoded in the stored PHC string, bounded by the backend's verification
/// cost limits.
///
/// # Verification cost limits
///
/// A stored PHC string chooses the memory, time, and parallelism costs of its
/// own verification. Without a bound, a single forged or corrupted record such
/// as `m=4294967295` makes every login attempt against it allocate terabytes or
/// spin for hours. The backend therefore refuses, before doing any work, to
/// verify a hash whose costs exceed its limits.
///
/// Each limit defaults to the larger of the configured Argon2 parameter and the
/// matching `DEFAULT_MAX_VERIFY_*` constant, so hashes produced by this backend
/// always verify. Use [`with_verification_limits`](Self::with_verification_limits)
/// when migrating from stored hashes with heavier settings.
#[derive(Clone)]
pub struct DefaultPassowrdHasher {
    instance: Argon2<'static>,
    max_m_cost: u32,
    max_t_cost: u32,
    max_p_cost: u32,
}

impl DefaultPassowrdHasher {
    /// Default ceiling for a stored hash's memory cost, in KiB (256 MiB).
    pub const DEFAULT_MAX_VERIFY_M_COST: u32 = 256 * 1024;

    /// Default ceiling for a stored hash's time cost (iterations).
    pub const DEFAULT_MAX_VERIFY_T_COST: u32 = 16;

    /// Default ceiling for a stored hash's parallelism (lanes).
    pub const DEFAULT_MAX_VERIFY_P_COST: u32 = 16;

    /// Creates a backend using the supplied Argon2 configuration.
    ///
    /// The instance contains the algorithm, version, and parameters; no separate
    /// parameter copy is needed. Verification limits are raised to cover the
    /// supplied parameters when those exceed the defaults.
    pub fn new(instance: Argon2<'static>) -> Self {
        let params = instance.params();
        let max_m_cost = params.m_cost().max(Self::DEFAULT_MAX_VERIFY_M_COST);
        let max_t_cost = params.t_cost().max(Self::DEFAULT_MAX_VERIFY_T_COST);
        let max_p_cost = params.p_cost().max(Self::DEFAULT_MAX_VERIFY_P_COST);

        Self {
            instance,
            max_m_cost,
            max_t_cost,
            max_p_cost,
        }
    }

    /// Replaces the largest costs a stored hash may request during verification.
    ///
    /// `max_m_cost` is in KiB. Hashes exceeding any limit are rejected with an
    /// error instead of being computed. Limits below the backend's own hashing
    /// parameters make its freshly created hashes unverifiable.
    #[must_use]
    pub fn with_verification_limits(
        mut self,
        max_m_cost: u32,
        max_t_cost: u32,
        max_p_cost: u32,
    ) -> Self {
        self.max_m_cost = max_m_cost;
        self.max_t_cost = max_t_cost;
        self.max_p_cost = max_p_cost;
        self
    }

    /// Rejects stored hashes whose Argon2 costs exceed the verification limits.
    ///
    /// Hashes whose parameters cannot be read as Argon2 parameters are left to
    /// the verifier, which fails them without performing any hashing work.
    fn check_verification_cost(&self, hash: &PasswordHash<'_>) -> PasswordHasherResult<()> {
        let Ok(params) = Params::try_from(hash) else {
            return Ok(());
        };

        if params.m_cost() > self.max_m_cost
            || params.t_cost() > self.max_t_cost
            || params.p_cost() > self.max_p_cost
        {
            return Err(PasswordHasherError::ParamValueInvalid(
                InvalidValue::TooLong,
            ));
        }

        Ok(())
    }
}

impl Default for DefaultPassowrdHasher {
    fn default() -> Self {
        Self::new(Argon2::default())
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
        self.check_verification_cost(&parsed_hash)?;
        Ok(self
            .instance
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}
