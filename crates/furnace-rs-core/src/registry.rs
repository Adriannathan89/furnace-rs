//! Application-scoped provider storage and typed lookup.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::{Diagnostic, Error, FURNACE001, FURNACE003, FURNACE004, Result};

/// A type-erased application-scoped provider allocation.
pub type ErasedProvider = Arc<dyn Any + Send + Sync>;

/// Stores at most one application-scoped provider for each concrete type.
pub struct ProviderRegistry {
    values: HashMap<TypeId, ErasedProvider>,
}

impl ProviderRegistry {
    /// Creates an empty provider registry.
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Stores a provider under its concrete type.
    #[allow(clippy::result_large_err)]
    pub fn insert<T>(&mut self, value: T) -> Result<()>
    where
        T: Send + Sync + 'static,
    {
        self.insert_erased(
            TypeId::of::<T>(),
            std::any::type_name::<T>(),
            Arc::new(value),
        )
    }

    /// Stores an erased provider under an explicit type identifier.
    #[allow(clippy::result_large_err)]
    pub fn insert_erased(
        &mut self,
        type_id: TypeId,
        type_name: &'static str,
        value: ErasedProvider,
    ) -> Result<()> {
        if self.values.contains_key(&type_id) {
            return Err(Error::new(
                Diagnostic::new(
                    FURNACE001,
                    "duplicate provider",
                    "an application-scoped provider is already registered for this type",
                )
                .with_subject(type_name),
            ));
        }

        self.values.insert(type_id, value);
        Ok(())
    }

    /// Resolves a provider by its concrete type.
    #[allow(clippy::result_large_err)]
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();
        let provider = self
            .values
            .get(&TypeId::of::<T>())
            .cloned()
            .ok_or_else(|| {
                Error::new(
                    Diagnostic::new(
                        FURNACE003,
                        "missing provider",
                        "no application-scoped provider is registered for this type",
                    )
                    .with_subject(type_name),
                )
            })?;

        Arc::downcast::<T>(provider).map_err(|_| {
            Error::new(
                Diagnostic::new(
                    FURNACE004,
                    "provider type mismatch",
                    "the provider registry contains a value that does not match its type identifier",
                )
                .with_subject(type_name),
            )
        })
    }

    /// Returns whether a provider is registered for the concrete type.
    pub fn contains<T>(&self) -> bool
    where
        T: Send + Sync + 'static,
    {
        self.values.contains_key(&TypeId::of::<T>())
    }

    /// Reports a dynamically selected integration output without resolving it.
    #[doc(hidden)]
    pub fn contains_type_id(&self, type_id: TypeId) -> bool {
        self.values.contains_key(&type_id)
    }

    /// Returns the number of registered providers.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns whether no providers are registered.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
