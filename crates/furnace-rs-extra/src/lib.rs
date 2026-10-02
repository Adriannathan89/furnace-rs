//! Post-v1 capability boundary for furnace-rs.
//!
//! The `furnace_rs foundation` command reports this boundary as reserved because its
//! scheduled extension APIs are not implemented in v0.2.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

/// Exposes the framework-neutral core boundary to future extensions.
pub use furnace_rs_core as core;
