//! Application lifecycle hooks, state transitions, and failure rollback.

use std::future::Future;
use std::pin::Pin;

use crate::{ApplicationContext, Diagnostic, Error, FURNACE010, FURNACE011, Result};

enum HookGroup {
    Infrastructure(&'static str),
    Application,
}

struct RegisteredHook {
    group: HookGroup,
    sequence: usize,
    hook: Box<dyn LifecycleHook>,
}

/// The lifecycle state of an application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleState {
    /// The application has been built but has not started.
    Created,
    /// The application is starting lifecycle hooks.
    Starting,
    /// The application has started successfully.
    Running,
    /// The application is stopping lifecycle hooks.
    Stopping,
    /// The application has stopped and cannot be restarted.
    Stopped,
}

/// The asynchronous result returned by a lifecycle hook.
pub type LifecycleFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

/// Starts and stops an application resource.
///
/// Hook futures can be dropped when their caller is cancelled. `start` must clean
/// up any partial work if it is dropped or returns an error: only a successful
/// start is registered for shutdown. `stop` must be safe to retry after its future
/// is dropped, because interrupted cleanup invokes it again. The manager retains
/// completed starts and stops, but cannot undo side effects inside a pending hook.
pub trait LifecycleHook: Send + Sync {
    /// Returns a stable name used in lifecycle diagnostics.
    fn name(&self) -> &str;

    /// Starts this resource using the completed application context.
    fn start<'a>(&'a self, context: &'a ApplicationContext) -> LifecycleFuture<'a>;

    /// Stops this resource using the completed application context.
    fn stop<'a>(&'a self, context: &'a ApplicationContext) -> LifecycleFuture<'a>;
}

pub(crate) enum LifecycleRegistration {
    Infrastructure {
        owner: &'static str,
        hook: Box<dyn LifecycleHook>,
    },
    Application {
        hook: Box<dyn LifecycleHook>,
    },
}

/// A native provider value accompanied by lifecycle hooks registered for it.
pub struct LifecycleResource<T> {
    value: T,
    registrations: Vec<LifecycleRegistration>,
}

impl<T> LifecycleResource<T> {
    /// Creates a lifecycle resource without any hooks.
    pub fn new(value: T) -> Self {
        Self {
            value,
            registrations: Vec::new(),
        }
    }

    /// Adds an infrastructure hook owned by the named framework component.
    #[must_use]
    pub fn with_infrastructure_hook<H>(mut self, owner: &'static str, hook: H) -> Self
    where
        H: LifecycleHook + 'static,
    {
        self.registrations
            .push(LifecycleRegistration::Infrastructure {
                owner,
                hook: Box::new(hook),
            });

        self
    }

    /// Adds an application hook after previously authored registrations.
    #[must_use]
    pub fn with_application_hook<H>(mut self, hook: H) -> Self
    where
        H: LifecycleHook + 'static,
    {
        self.registrations.push(LifecycleRegistration::Application {
            hook: Box::new(hook),
        });

        self
    }

    /// Separates the native value from registrations for the application builder.
    pub(crate) fn into_parts(self) -> (T, Vec<LifecycleRegistration>) {
        (self.value, self.registrations)
    }
}

/// Coordinates ordered lifecycle hooks for one application.
pub struct LifecycleManager {
    state: LifecycleState,
    hooks: Vec<RegisteredHook>,
    next_sequence: usize,
    started: Vec<usize>,
    rollback_failure: Option<(Diagnostic, Error)>,
    shutdown_failure: Option<Error>,
}

impl LifecycleManager {
    /// Creates a manager in the created state with no lifecycle hooks.
    pub fn new() -> Self {
        Self {
            state: LifecycleState::Created,
            hooks: Vec::new(),
            next_sequence: 0,
            started: Vec::new(),
            rollback_failure: None,
            shutdown_failure: None,
        }
    }

    /// Adds a lifecycle hook that starts after previously registered hooks.
    pub fn add_hook<H>(&mut self, hook: H) -> &mut Self
    where
        H: LifecycleHook + 'static,
    {
        self.add_registration(LifecycleRegistration::Application {
            hook: Box::new(hook),
        });

        self
    }

    /// Adds an infrastructure hook owned by the named framework component.
    #[doc(hidden)]
    pub fn add_infrastructure_hook<H>(&mut self, owner: &'static str, hook: H) -> &mut Self
    where
        H: LifecycleHook + 'static,
    {
        self.add_registration(LifecycleRegistration::Infrastructure {
            owner,
            hook: Box::new(hook),
        });

        self
    }

    /// Adds an already-erased infrastructure hook without changing its owner label.
    pub(crate) fn add_boxed_infrastructure_hook(
        &mut self,
        owner: &'static str,
        hook: Box<dyn LifecycleHook>,
    ) {
        self.add_registration(LifecycleRegistration::Infrastructure { owner, hook });
    }

    /// Assigns sequence metadata and appends one hook to its lifecycle group.
    pub(crate) fn add_registration(&mut self, registration: LifecycleRegistration) {
        let (group, hook) = match registration {
            LifecycleRegistration::Infrastructure { owner, hook } => {
                (HookGroup::Infrastructure(owner), hook)
            }
            LifecycleRegistration::Application { hook } => (HookGroup::Application, hook),
        };
        self.hooks.push(RegisteredHook {
            group,
            sequence: self.next_sequence,
            hook,
        });
        self.next_sequence += 1;
    }

    /// Returns the manager's current lifecycle state.
    pub const fn state(&self) -> LifecycleState {
        self.state
    }

    /// Starts infrastructure hooks by owner before application hooks in registration order.
    ///
    /// If this future is dropped, call [`Self::shutdown`] to clean up hooks that
    /// completed startup. Startup cannot be retried on this manager.
    #[allow(clippy::result_large_err)]
    pub async fn start(&mut self, context: &ApplicationContext) -> Result<()> {
        if self.state != LifecycleState::Created {
            return Err(invalid_transition(self.state, "start"));
        }

        self.state = LifecycleState::Starting;
        self.hooks
            .sort_by(|left, right| match (&left.group, &right.group) {
                (HookGroup::Infrastructure(left_owner), HookGroup::Infrastructure(right_owner)) => {
                    left_owner
                        .cmp(right_owner)
                        .then(left.sequence.cmp(&right.sequence))
                }
                (HookGroup::Infrastructure(_), HookGroup::Application) => std::cmp::Ordering::Less,
                (HookGroup::Application, HookGroup::Infrastructure(_)) => {
                    std::cmp::Ordering::Greater
                }
                (HookGroup::Application, HookGroup::Application) => {
                    left.sequence.cmp(&right.sequence)
                }
            });

        for index in 0..self.hooks.len() {
            let hook = self.hooks[index].hook.as_ref();

            if let Err(error) = hook.start(context).await {
                self.rollback_failure =
                    Some((hook_failure_diagnostic(hook.name(), "startup"), error));

                return self.shutdown(context).await;
            }

            self.started.push(index);
        }

        self.state = LifecycleState::Running;

        Ok(())
    }

    /// Stops hooks in reverse successful startup order, including interrupted startup.
    ///
    /// A dropped shutdown or rollback future can be resumed by calling this
    /// method again. Completed stops are not repeated; the pending stop is retried.
    /// Earlier startup or shutdown failures are retained across cancellation.
    #[allow(clippy::result_large_err)]
    pub async fn shutdown(&mut self, context: &ApplicationContext) -> Result<()> {
        if !matches!(
            self.state,
            LifecycleState::Starting | LifecycleState::Running | LifecycleState::Stopping
        ) {
            return Err(invalid_transition(self.state, "shutdown"));
        }

        self.state = LifecycleState::Stopping;

        while let Some(&index) = self.started.last() {
            let hook = self.hooks[index].hook.as_ref();

            if let Err(error) = hook.stop(context).await {
                if let Some((diagnostic, _)) = &mut self.rollback_failure {
                    *diagnostic = diagnostic
                        .clone()
                        .with_suggestion(format!("rollback hook {} failed: {error}", hook.name()));
                } else if self.shutdown_failure.is_none() {
                    self.shutdown_failure = Some(hook_failure(hook.name(), "shutdown", error));
                }
            }

            // Keep the pending hook reachable until its stop future completes.
            self.started.pop();
        }

        self.state = LifecycleState::Stopped;

        if let Some((diagnostic, error)) = self.rollback_failure.take() {
            return Err(Error::with_source(diagnostic, error));
        }

        self.shutdown_failure.take().map_or(Ok(()), Err)
    }
}

impl Default for LifecycleManager {
    /// Creates the same empty, not-yet-started manager as [`Self::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Reports a start or shutdown call that is invalid for the current state.
fn invalid_transition(state: LifecycleState, operation: &str) -> Error {
    Error::new(
        Diagnostic::new(
            FURNACE010,
            "invalid lifecycle transition",
            format!("cannot {operation} an application while it is {state:?}"),
        )
        .with_subject(operation),
    )
}

/// Retains a hook's structured failure under the stable lifecycle diagnostic.
fn hook_failure(name: &str, operation: &str, error: Error) -> Error {
    Error::with_source(hook_failure_diagnostic(name, operation), error)
}

/// Creates the safe diagnostic shared by startup rollback and shutdown errors.
fn hook_failure_diagnostic(name: &str, operation: &str) -> Diagnostic {
    Diagnostic::new(
        FURNACE011,
        "lifecycle hook failed",
        format!("lifecycle hook failed during {operation}"),
    )
    .with_subject(name)
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::*;
    use crate::ProviderContribution;

    struct Value;

    struct NamedHook(&'static str);

    impl LifecycleHook for NamedHook {
        fn name(&self) -> &str {
            self.0
        }

        fn start<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
            Box::pin(async { Ok(()) })
        }

        fn stop<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
            Box::pin(async { Ok(()) })
        }
    }

    #[test]
    fn resource_preserves_hook_order_and_erases_the_native_value() {
        let resource = LifecycleResource::new(Value)
            .with_infrastructure_hook("zeta", NamedHook("first"))
            .with_application_hook(NamedHook("second"))
            .with_infrastructure_hook("alpha", NamedHook("third"));

        let (provider, registrations) = ProviderContribution::from_resource(resource).into_parts();

        assert_eq!(provider.as_ref().type_id(), TypeId::of::<Value>());
        assert_eq!(registrations.len(), 3);
        assert!(matches!(
            &registrations[0],
            LifecycleRegistration::Infrastructure { owner: "zeta", hook }
                if hook.name() == "first"
        ));
        assert!(matches!(
            &registrations[1],
            LifecycleRegistration::Application { hook } if hook.name() == "second"
        ));
        assert!(matches!(
            &registrations[2],
            LifecycleRegistration::Infrastructure { owner: "alpha", hook }
                if hook.name() == "third"
        ));
    }
}
