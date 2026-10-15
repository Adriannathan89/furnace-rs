//! Deterministic traversal for supported nested collection inputs.

use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

use super::{
    Input, ValidationPathSegment, ValidationResult,
    support::{finish, merge},
};

impl<T: Input> Input for Option<T> {
    /// Validates the contained value when present and accepts `None`.
    fn validate(&self) -> ValidationResult {
        self.as_ref().map_or(Ok(()), Input::validate)
    }
}

impl<T: Input> Input for Vec<T> {
    /// Validates every element and prefixes failures with its vector index.
    fn validate(&self) -> ValidationResult {
        sequence(self.iter())
    }
}

impl<T: Input, const N: usize> Input for [T; N] {
    /// Validates every element and prefixes failures with its array index.
    fn validate(&self) -> ValidationResult {
        sequence(self.iter())
    }
}

/// Collects indexed validation failures while preserving sequence order.
fn sequence<'a, T: Input + 'a>(values: impl Iterator<Item = &'a T>) -> ValidationResult {
    let mut errors = None;
    for (index, value) in values.enumerate() {
        merge(
            &mut errors,
            value
                .validate()
                .map_err(|errors| errors.__prefix(&[ValidationPathSegment::Index(index)])),
        );
    }
    finish(errors)
}

impl<T: Input, S: BuildHasher> Input for HashMap<String, T, S> {
    /// Validates values in sorted-key order for deterministic diagnostics.
    fn validate(&self) -> ValidationResult {
        let mut values: Vec<_> = self.iter().collect();
        values.sort_unstable_by_key(|(key, _)| *key);
        mapping(values.into_iter())
    }
}

impl<T: Input> Input for BTreeMap<String, T> {
    /// Validates values in the map's natural sorted-key order.
    fn validate(&self) -> ValidationResult {
        mapping(self.iter())
    }
}

/// Collects map-value failures and prefixes each path with its string key.
fn mapping<'a, T: Input + 'a>(
    values: impl Iterator<Item = (&'a String, &'a T)>,
) -> ValidationResult {
    let mut errors = None;
    for (key, value) in values {
        merge(
            &mut errors,
            value
                .validate()
                .map_err(|errors| errors.__prefix(&[ValidationPathSegment::Field(key.clone())])),
        );
    }
    finish(errors)
}
