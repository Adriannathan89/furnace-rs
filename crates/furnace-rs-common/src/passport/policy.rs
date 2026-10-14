//! Static, transport-independent Passport guard policies and validation.
use super::{ErasedAuthentication, PassportContext, PassportStrategyFuture};
use furnace_rs_core::{Diagnostic, Error, Result, SourceLocation};
use std::{any::TypeId, fmt};

/// The one token source selected by a Passport guard.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum TokenSource {
    /// Read one RFC 6750 Bearer credential from the `Authorization` header.
    Bearer,
    /// Read one strict request cookie by its literal name.
    #[cfg(feature = "cookies")]
    Cookie(&'static str),
}

impl fmt::Debug for TokenSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bearer => formatter.write_str("Bearer"),
            // Cookie names can identify a deployment's authentication surface;
            // diagnostics intentionally retain only the source category.
            #[cfg(feature = "cookies")]
            Self::Cookie(_) => formatter.write_str("Cookie(..)"),
        }
    }
}

/// The matching rule applied to a role or permission collection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PolicyMode {
    /// At least one configured value must match.
    Any,
    /// Every configured value must match.
    All,
}

/// One role or permission policy clause emitted by `#[guard]`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PolicyClause {
    mode: PolicyMode,
    values: &'static [&'static str],
}

impl PolicyClause {
    /// Creates one static policy clause.
    #[must_use]
    pub const fn new(mode: PolicyMode, values: &'static [&'static str]) -> Self {
        Self { mode, values }
    }

    /// Returns whether this clause uses `any` or `all` matching.
    #[must_use]
    pub const fn mode(self) -> PolicyMode {
        self.mode
    }

    /// Returns the configured role or permission values.
    #[must_use]
    pub const fn values(self) -> &'static [&'static str] {
        self.values
    }
}

/// A generated, type-safe policy predicate over an erased authentication.
pub type GuardPredicateAdapter = fn(&ErasedAuthentication) -> bool;

/// One named predicate emitted by `#[guard]`.
#[derive(Clone, Copy)]
pub struct GuardPredicate {
    name: &'static str,
    adapter: Option<GuardPredicateAdapter>,
}

impl GuardPredicate {
    /// Creates static predicate metadata.
    ///
    /// `None` is accepted solely so manually supplied metadata can be
    /// validated fail-closed by [`GuardCatalog::validate_descriptors`]. Macro-generated
    /// descriptors always provide an adapter.
    #[must_use]
    pub const fn new(name: &'static str, adapter: Option<GuardPredicateAdapter>) -> Self {
        Self { name, adapter }
    }

    /// Returns the source path recorded for this predicate.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns the generated type-safe adapter when metadata is valid.
    #[doc(hidden)]
    #[must_use]
    pub const fn adapter(self) -> Option<GuardPredicateAdapter> {
        self.adapter
    }
}

/// Framework adapter for the built-in `jwt` strategy supplied by a direct
/// `ClaimsPrincipal<C>` guard.
pub type BuiltinGuardAdapter = for<'a> fn(
    &'a furnace_rs_core::ApplicationContext,
    &'a PassportContext<'a>,
    &'a str,
) -> PassportStrategyFuture<'a>;

/// Immutable effective policy for one guarded route method.
pub struct GuardDescriptor {
    policy_name: &'static str,
    handler: &'static str,
    requirement_subject: &'static str,
    namespace: Option<&'static str>,
    strategy: &'static str,
    principal_type_id: Option<fn() -> TypeId>,
    principal_type_name: Option<fn() -> &'static str>,
    source: TokenSource,
    roles: Option<PolicyClause>,
    permissions: Option<PolicyClause>,
    predicates: &'static [GuardPredicate],
    location: SourceLocation,
    builtin_adapter: Option<BuiltinGuardAdapter>,
}

/// Provides immutable policy metadata without creating a guard value.
#[doc(hidden)]
pub trait GuardPolicy: Send + Sync + 'static {
    /// Returns this policy's generated static descriptor.
    fn descriptor() -> &'static GuardDescriptor;
}

impl GuardDescriptor {
    /// Creates static effective guard metadata.
    ///
    /// The optional principal factories permit integrations to build
    /// descriptors manually. [`GuardCatalog::validate_descriptors`] rejects omitted or
    /// inconsistent factories before startup.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        policy_name: &'static str,
        handler: &'static str,
        strategy: &'static str,
        principal_type_id: Option<fn() -> TypeId>,
        principal_type_name: Option<fn() -> &'static str>,
        source: TokenSource,
        roles: Option<PolicyClause>,
        permissions: Option<PolicyClause>,
        predicates: &'static [GuardPredicate],
        location: SourceLocation,
        builtin_adapter: Option<BuiltinGuardAdapter>,
    ) -> Self {
        Self {
            policy_name,
            handler,
            requirement_subject: "manual Passport guard",
            namespace: None,
            strategy,
            principal_type_id,
            principal_type_name,
            source,
            roles,
            permissions,
            predicates,
            location,
            builtin_adapter,
        }
    }

    /// Attaches the stable requirement subject emitted by `#[guard]`.
    ///
    /// Route expansion supplies `RouteTrait::method` with `concat!`, keeping
    /// auto-configuration evidence static and free of application values.
    #[doc(hidden)]
    #[must_use]
    pub const fn with_requirement_subject(mut self, subject: &'static str) -> Self {
        self.requirement_subject = subject;
        self
    }

    /// Attaches the Rust namespace containing this guard declaration.
    #[doc(hidden)]
    #[must_use]
    pub const fn with_namespace(mut self, namespace: &'static str) -> Self {
        self.namespace = Some(namespace);
        self
    }

    /// Returns the Rust namespace containing this guard declaration, when available.
    #[doc(hidden)]
    pub const fn namespace(&self) -> Option<&'static str> {
        self.namespace
    }

    /// Returns the static policy declaration name.
    #[must_use]
    pub const fn policy_name(&self) -> &'static str {
        self.policy_name
    }

    /// Returns the policy declaration label (`seal` for generated policies).
    #[must_use]
    pub const fn handler(&self) -> &'static str {
        self.handler
    }

    /// Returns the stable subject used for JWT auto-configuration evidence.
    #[doc(hidden)]
    #[must_use]
    pub const fn requirement_subject(&self) -> &'static str {
        self.requirement_subject
    }

    /// Returns the requested Passport strategy name.
    #[must_use]
    pub const fn strategy(&self) -> &'static str {
        self.strategy
    }

    /// Returns the exact principal type identifier when present.
    #[must_use]
    pub fn principal_type_id(&self) -> Option<TypeId> {
        self.principal_type_id.map(|factory| factory())
    }

    /// Returns the exact principal type name when present.
    #[must_use]
    pub fn principal_type_name(&self) -> Option<&'static str> {
        self.principal_type_name.map(|factory| factory())
    }

    /// Returns the one token source used for this guard.
    #[must_use]
    pub const fn source(&self) -> TokenSource {
        self.source
    }

    /// Returns the optional roles clause.
    #[must_use]
    pub const fn roles(&self) -> Option<PolicyClause> {
        self.roles
    }

    /// Returns the optional permissions clause.
    #[must_use]
    pub const fn permissions(&self) -> Option<PolicyClause> {
        self.permissions
    }

    /// Returns every ANDed custom predicate in declaration order.
    #[must_use]
    pub const fn predicates(&self) -> &'static [GuardPredicate] {
        self.predicates
    }

    /// Returns the route declaration source location.
    #[must_use]
    pub const fn location(&self) -> SourceLocation {
        self.location
    }

    /// Returns a built-in typed-claims adapter when this guard can use `jwt`
    /// without a custom managed strategy.
    #[doc(hidden)]
    #[must_use]
    pub const fn builtin_adapter(&self) -> Option<BuiltinGuardAdapter> {
        self.builtin_adapter
    }
}

/// Read-only inspection and validation of linked Passport route guards.
pub struct GuardCatalog;

impl GuardCatalog {
    /// Validates an explicit static descriptor slice without registering it.
    ///
    /// This permits integration crates to validate manually assembled
    /// descriptors using the same fail-closed rules as inventory metadata.
    #[doc(hidden)]
    #[allow(clippy::result_large_err)]
    pub fn validate_descriptors(descriptors: &[&GuardDescriptor]) -> Result<()> {
        for guard in descriptors {
            validate_guard(guard)?;
        }
        Ok(())
    }
}

pub(crate) fn authorizes(guard: &GuardDescriptor, authentication: &ErasedAuthentication) -> bool {
    let principal = authentication.principal();
    if !matches_clause(guard.roles(), |value| principal.has_role(value))
        || !matches_clause(guard.permissions(), |value| principal.has_permission(value))
    {
        return false;
    }
    guard.predicates().iter().all(|predicate| {
        predicate
            .adapter()
            .is_some_and(|adapter| adapter(authentication))
    })
}

fn matches_clause(clause: Option<PolicyClause>, matches: impl FnMut(&str) -> bool) -> bool {
    let Some(clause) = clause else {
        return true;
    };
    match clause.mode() {
        PolicyMode::Any => clause.values().iter().copied().any(matches),
        PolicyMode::All => clause.values().iter().copied().all(matches),
    }
}

fn validate_guard(guard: &GuardDescriptor) -> Result<()> {
    let subject = guard_subject(guard);
    if guard.policy_name().is_empty() || guard.handler().is_empty() {
        return Err(metadata_error(
            subject,
            "policy and declaration names must be present",
            guard.location(),
        ));
    }
    if !valid_strategy_name(guard.strategy()) {
        return Err(metadata_error(
            subject,
            "guard strategy name must be non-empty lowercase ASCII `[a-z0-9._-]`",
            guard.location(),
        ));
    }
    if guard.principal_type_id().is_none() || guard.principal_type_name().is_none_or(str::is_empty)
    {
        return Err(metadata_error(
            subject,
            "guard principal type metadata must be present",
            guard.location(),
        ));
    }
    #[cfg(feature = "cookies")]
    if let TokenSource::Cookie(name) = guard.source()
        && !valid_cookie_name(name)
    {
        return Err(metadata_error(
            subject,
            "guard cookie token source must use a non-empty RFC cookie name",
            guard.location(),
        ));
    }
    for (label, clause) in [
        ("roles", guard.roles()),
        ("permissions", guard.permissions()),
    ] {
        if let Some(clause) = clause
            && (clause.values().is_empty()
                || clause
                    .values()
                    .iter()
                    .any(|value| value.is_empty() || value.chars().any(char::is_control)))
        {
            return Err(metadata_error(
                subject,
                format!("guard {label} policy must contain non-empty values"),
                guard.location(),
            ));
        }
    }
    if guard
        .predicates()
        .iter()
        .any(|predicate| predicate.name().is_empty() || predicate.adapter().is_none())
    {
        return Err(metadata_error(
            subject,
            "guard predicates must have a name and generated adapter",
            guard.location(),
        ));
    }
    if guard.location().file.is_empty()
        || guard.location().line == 0
        || guard.location().column == 0
    {
        return Err(metadata_error(
            subject,
            "guard source file, line, and column must be present",
            guard.location(),
        ));
    }
    Ok(())
}

fn guard_subject(guard: &GuardDescriptor) -> String {
    format!("{}::{}", guard.policy_name(), guard.handler())
}

fn valid_strategy_name(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

#[cfg(feature = "cookies")]
fn valid_cookie_name(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

fn metadata_error(subject: String, message: impl Into<String>, location: SourceLocation) -> Error {
    Error::new(
        Diagnostic::new(
            super::FURNACE131,
            "invalid Passport guard metadata",
            message,
        )
        .with_subject(subject)
        .with_location(location)
        .with_suggestion("correct the guard metadata before starting the application"),
    )
}
