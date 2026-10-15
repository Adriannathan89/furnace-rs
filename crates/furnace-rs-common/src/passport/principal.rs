//! Principal policy and guarded-request extractor contracts.

use std::{any::type_name, fmt, ops::Deref, sync::Arc};

use crate::VerifiedJwt;

/// An authenticated application identity that can answer guard policies.
pub trait PassportPrincipal: Send + Sync + 'static {
    /// Returns whether this principal has `role`.
    fn has_role(&self, role: &str) -> bool;

    /// Returns whether this principal has `permission`.
    fn has_permission(&self, permission: &str) -> bool;
}

/// A principal backed directly by a complete verified JWT.
pub struct ClaimsPrincipal<C> {
    verified: Arc<VerifiedJwt<C>>,
}

impl<C> ClaimsPrincipal<C> {
    /// Creates a principal from one complete verified JWT.
    ///
    /// This is used by the generated built-in `jwt` guard adapter. Application
    /// code normally receives this type through [`Authenticated`].
    #[doc(hidden)]
    #[must_use]
    pub fn new(verified: Arc<VerifiedJwt<C>>) -> Self {
        Self { verified }
    }

    /// Borrows the complete verified token retained by this principal.
    #[must_use]
    pub fn verified(&self) -> &VerifiedJwt<C> {
        &self.verified
    }
}

impl<C> Clone for ClaimsPrincipal<C> {
    /// Clones the shared reference to the verified token.
    fn clone(&self) -> Self {
        Self {
            verified: Arc::clone(&self.verified),
        }
    }
}

impl<C> Deref for ClaimsPrincipal<C> {
    type Target = C;

    /// Borrows the application-owned custom claims stored in the verified token.
    fn deref(&self) -> &Self::Target {
        &self.verified.claims.custom
    }
}

impl<C> fmt::Debug for ClaimsPrincipal<C> {
    /// Formats the custom-claims type without printing claims data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClaimsPrincipal")
            .field("claims_type", &type_name::<C>())
            .finish()
    }
}

impl<C> PassportPrincipal for ClaimsPrincipal<C>
where
    C: PassportPrincipal,
{
    /// Delegates role checks to the custom claims value.
    fn has_role(&self, role: &str) -> bool {
        self.verified.claims.custom.has_role(role)
    }

    /// Delegates permission checks to the custom claims value.
    fn has_permission(&self, permission: &str) -> bool {
        self.verified.claims.custom.has_permission(permission)
    }
}

/// The typed principal installed by a successful guard.
///
/// Use HTTP extraction or `grpc_authenticated` with the corresponding transport.
pub struct Authenticated<P>(Arc<P>);

impl<P> Authenticated<P> {
    #[allow(dead_code)]
    /// Wraps the principal shared by a successful guard for request extraction.
    pub(crate) const fn new(principal: Arc<P>) -> Self {
        Self(principal)
    }
}

impl<P> Clone for Authenticated<P> {
    /// Clones the shared reference to the authenticated principal.
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<P> Deref for Authenticated<P> {
    type Target = P;

    /// Borrows the authenticated application principal.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P> fmt::Debug for Authenticated<P> {
    /// Formats the principal type name without invoking its value formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Authenticated")
            .field("principal_type", &type_name::<P>())
            .finish()
    }
}

/// The complete verified token installed by a successful guard.
pub struct VerifiedToken<C>(Arc<VerifiedJwt<C>>);

impl<C> VerifiedToken<C> {
    #[allow(dead_code)]
    /// Wraps a verified token for extraction from a successfully guarded request.
    pub(crate) const fn new(verified: Arc<VerifiedJwt<C>>) -> Self {
        Self(verified)
    }

    /// Borrows the complete verified token.
    #[must_use]
    pub fn verified(&self) -> &VerifiedJwt<C> {
        &self.0
    }
}

impl<C> Clone for VerifiedToken<C> {
    /// Clones the shared reference to the verified token.
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<C> Deref for VerifiedToken<C> {
    type Target = VerifiedJwt<C>;

    /// Borrows the complete verified JWT.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C> fmt::Debug for VerifiedToken<C> {
    /// Formats the claims type without printing custom claim values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedToken")
            .field("claims_type", &type_name::<C>())
            .finish()
    }
}
