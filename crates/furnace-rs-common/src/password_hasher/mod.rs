//! Replaceable password hashing services with an Argon2 default backend.
//!
//! Enabled by the `password-hasher` feature. The helpers are also re-exported
//! from the crate root. Each hash uses a fresh OS-generated salt and is serialized
//! as a PHC string containing the algorithm, version, parameters, salt, and hash.
//! Store the complete string so verification can recover those settings.
//!
//! The default backend and free helpers return an error for malformed PHC
//! strings. Once parsing succeeds, every verification failure (including a
//! password mismatch or unsupported algorithm) becomes `Ok(false)`.
//!
//! # Direct use without a cauldron
//!
//! Importing [`PasswordHasherCauldron`] is optional. Create a [`PasswordHasher`]
//! directly when passing it to functions yourself. It stores its backend in an
//! [`Arc`](std::sync::Arc); cloning the hasher shares the same backend rather
//! than copying it. Each separate call to [`PasswordHasher::default`] creates a
//! new backend.
//!
//! ```
//! use furnace_rs_common::{PasswordHasher, PasswordHasherResult};
//!
//! fn create_hash(hasher: PasswordHasher, password: &str) -> PasswordHasherResult<String> {
//!     hasher.hash_password(password)
//! }
//!
//! fn check_password(
//!     hasher: PasswordHasher,
//!     password: &str,
//!     hashed: &str,
//! ) -> PasswordHasherResult<bool> {
//!     hasher.validate_password(password, hashed)
//! }
//!
//! let hasher = PasswordHasher::default();
//! let hashed = create_hash(hasher.clone(), "secret")?;
//! assert!(check_password(hasher.clone(), "secret", &hashed)?);
//! # Ok::<(), furnace_rs_common::PasswordHasherError>(())
//! ```
//!
//! Import [`PasswordHasherCauldron`] into your application's cauldron when you
//! want Furnace to inject the default hasher into services instead of passing
//! the instance manually.

mod default;
mod password_hasher_service;

pub use default::DefaultPassowrdHasher;
pub use password_hasher_service::{PasswordHasher, PasswordHasherService};

/// Error returned by password hashing and validation backends.
pub use argon2::password_hash::Error as PasswordHasherError;

/// Result returned by password hashing and validation backends.
pub type PasswordHasherResult<T> = Result<T, PasswordHasherError>;

/// Global module providing the default application [`PasswordHasher`].
///
/// This cauldron is optional for direct use through [`PasswordHasher::default`]
/// or [`PasswordHasher::new`].
///
/// Import this cauldron into the application root to make the hasher available
/// to consumers. To replace the backend, supply [`PasswordHasher::new`] through
/// the application builder while keeping the provider registered and exported.
///
/// ```
/// use furnace_rs_common::{PasswordHasher, PasswordHasherCauldron};
///
/// #[furnace_rs_common::core::burner]
/// struct AuthService {
///     #[inject]
///     hasher: PasswordHasher,
/// }
///
/// #[furnace_rs_common::core::cauldron(
///     imports = [PasswordHasherCauldron],
///     providers = [AuthService],
/// )]
/// struct AppCauldron;
/// ```
#[crate::core::cauldron(providers = [PasswordHasher], exports = [PasswordHasher], global)]
pub struct PasswordHasherCauldron;

impl crate::core::Injector for PasswordHasher {
    type Dependencies = ();

    async fn inject((): ()) -> crate::core::Result<Self> {
        Ok(Self::default())
    }

    fn descriptor() -> &'static crate::core::ProviderDescriptor {
        &PASSWORD_HASHER_DESCRIPTOR
    }
}

const PASSWORD_HASHER_DESCRIPTOR: crate::core::ProviderDescriptor =
    crate::core::__private::InjectorMetadata::<PasswordHasher, PasswordHasher>::DESCRIPTOR
        .with_authored_type_name("PasswordHasher")
        .with_visibility(crate::core::ProviderVisibility::Public)
        .with_namespace(module_path!())
        .with_location(crate::core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));

crate::core::__private::inventory::submit! { PASSWORD_HASHER_DESCRIPTOR }

/// Hashes the password's UTF-8 bytes with Argon2 defaults and a fresh random salt.
///
/// Returns the complete PHC string, suitable for storage. Repeated calls for the
/// same password produce independently salted hashes.
///
/// # Errors
///
/// Returns the underlying error if Argon2 cannot hash or encode the password.
/// Salt generation uses OS randomness and can panic if that source fails.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    DefaultPassowrdHasher::default().hash_password(password)
}

/// Checks a password against a stored PHC string using its encoded settings.
///
/// Returns `Ok(true)` on a match and `Ok(false)` on any verification failure,
/// including an incorrect password or an unsupported algorithm.
///
/// # Errors
///
/// Returns the parsing error when `hashed` is not a valid PHC string.
pub fn validate_password(
    password: &str,
    hashed: &str,
) -> Result<bool, argon2::password_hash::Error> {
    DefaultPassowrdHasher::default().validate_password(password, hashed)
}

#[cfg(test)]
mod tests {
    use super::{hash_password, validate_password};
    use argon2::password_hash::PasswordHash;

    #[test]
    fn creates_an_argon2id_phc_string() {
        let hashed = hash_password("correct horse battery staple").expect("hash password");
        let parsed = PasswordHash::new(&hashed).expect("parse generated PHC string");

        assert_eq!(parsed.algorithm.as_str(), "argon2id");
        assert_eq!(parsed.version, Some(19));
        assert!(parsed.salt.is_some());
        assert!(parsed.hash.is_some());

        for parameter in ["m", "t", "p"] {
            assert!(parsed.params.get_decimal(parameter).is_some());
        }
    }

    #[test]
    fn validates_the_original_password() {
        let password = "correct horse battery staple";
        let hashed = hash_password(password).expect("hash password");

        assert_eq!(validate_password(password, &hashed), Ok(true));
    }

    #[test]
    fn rejects_an_incorrect_password() {
        let hashed = hash_password("correct password").expect("hash password");

        assert_eq!(validate_password("wrong password", &hashed), Ok(false));
    }

    #[test]
    fn creates_independently_salted_hashes_for_the_same_password() {
        let password = "same password";
        let first = hash_password(password).expect("hash first password");
        let second = hash_password(password).expect("hash second password");
        let first_parsed = PasswordHash::new(&first).expect("parse first hash");
        let second_parsed = PasswordHash::new(&second).expect("parse second hash");

        assert_ne!(first_parsed.salt, second_parsed.salt);
        assert_ne!(first, second);
        assert_eq!(validate_password(password, &first), Ok(true));
        assert_eq!(validate_password(password, &second), Ok(true));
    }

    #[test]
    fn returns_parsing_errors_for_malformed_hashes() {
        for malformed in ["", "not a PHC string", "$argon2id$v=invalid"] {
            let expected = PasswordHash::new(malformed).expect_err("invalid PHC string");
            assert_eq!(validate_password("password", malformed), Err(expected));
        }
    }

    #[test]
    fn returns_false_for_an_unsupported_algorithm() {
        let unsupported = "$unsupported$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$YWJjZGVmZ2hpamtsbW5vcA";
        PasswordHash::new(unsupported).expect("syntactically valid PHC string");

        assert_eq!(validate_password("password", unsupported), Ok(false));
    }

    #[test]
    fn supports_empty_and_unicode_passwords_without_normalizing_them() {
        for password in ["", "密碼🔐 café"] {
            let hashed = hash_password(password).expect("hash UTF-8 password");
            assert_eq!(validate_password(password, &hashed), Ok(true));
            assert_eq!(
                validate_password(&format!("{password} "), &hashed),
                Ok(false)
            );
        }

        let hashed = hash_password("é").expect("hash composed character");
        assert_eq!(validate_password("e\u{301}", &hashed), Ok(false));
    }
}
