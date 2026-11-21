//! The credential a Passport guard accepted, for per-client keys behind a guard.

use std::{fmt, sync::Arc};

/// The raw credential a Passport guard accepted for this request.
///
/// A guard installs it in the request extensions only after authentication
/// and authorization succeed, so layers behind a guard can key per-client
/// state on what was actually validated rather than on arbitrary headers.
#[derive(Clone)]
pub struct ValidatedCredential(Arc<str>);

impl ValidatedCredential {
    /// Wraps a validated credential. Installed by guards; public for test doubles.
    #[doc(hidden)]
    #[must_use]
    pub fn new(credential: &str) -> Self {
        Self(credential.into())
    }

    /// Borrows the credential. Never log it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ValidatedCredential {
    /// Redacts the credential.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedCredential(..)")
    }
}
