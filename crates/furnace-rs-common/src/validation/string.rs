//! String syntax and Unicode-aware length helpers.

use std::collections::{BTreeMap, HashMap};

/// Measures values according to the input validation length contract.
pub trait Length {
    /// Returns code-point length for strings or entry count for collections.
    fn length(&self) -> usize;
}

impl Length for str {
    /// Counts Unicode scalar values rather than UTF-8 bytes.
    fn length(&self) -> usize {
        self.chars().count()
    }
}

impl Length for String {
    /// Counts Unicode scalar values rather than UTF-8 bytes.
    fn length(&self) -> usize {
        self.chars().count()
    }
}

impl<T: Length + ?Sized> Length for &T {
    /// Delegates length calculation to the referenced value.
    fn length(&self) -> usize {
        (**self).length()
    }
}

impl<T> Length for Vec<T> {
    /// Returns the number of vector entries.
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T, const N: usize> Length for [T; N] {
    /// Returns the array's compile-time element count.
    fn length(&self) -> usize {
        N
    }
}

impl<T, S> Length for HashMap<String, T, S> {
    /// Returns the number of map entries.
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T> Length for BTreeMap<String, T> {
    /// Returns the number of map entries.
    fn length(&self) -> usize {
        self.len()
    }
}

/// Checks the same practical email policy used by typed configuration.
pub fn email(value: &str) -> bool {
    furnace_rs_core::__private::configuration_validation::email(value)
}
