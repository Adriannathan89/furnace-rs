//! Framework-neutral runtime contracts for furnace-rs.
//!
//! FURNACE core provides framework-neutral application construction, lifecycle
//! management, deterministic scalar TOML configuration, optional dotenv
//! interpolation, programmatic and environment sources, provider metadata, and
//! dependency graph analysis. Dotenv values only participate in interpolation:
//! they do not mutate process state, and real process variables take
//! precedence. Applications construct configuration explicitly; [`Furnace::builder`]
//! does not load files, dotenv variables, or process environment variables.
//!
//! Core also owns deterministic evaluation of official conditional defaults and
//! the public, retained [`AutoConfigurationReport`] records that explain their
//! decisions. Reports retain only stable identifiers, reason codes, and redacted
//! configuration evidence; they never retain resolved configuration values. The
//! declaration catalog is discovered statically; rooted builds select only explicitly
//! registered cauldron members and enforce imports and exports. Core re-exports the procedural macros
//! without introducing integration dependencies.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

extern crate self as furnace_rs_core;

mod auto_configuration;
mod builder;
mod catalog;
mod cauldron;
mod config;
mod configuration;
mod context;
mod descriptor;
mod diagnostic;
mod graph;
mod injector;
mod lifecycle;
mod preflight;
mod registry;
#[cfg(feature = "runtime-tokio")]
pub mod runtime;

pub use auto_configuration::{
    AutoConfigurationConfigEvidence, AutoConfigurationReasonCode, AutoConfigurationReport,
    AutoConfigurationRequirement, AutoConfigurationStatus,
};
pub use builder::{Furnace, FurnaceBuilder};
pub use catalog::Catalog;
#[doc(hidden)]
pub use cauldron::CauldronDefinition;
pub use cauldron::{Cauldron, CauldronRegistration};
pub use config::{
    Config, ConfigBuilder, ConfigDocument, ConfigSource, ConfigValue, DotenvSource, EnvSource,
    MapSource, TomlSource,
};
pub use configuration::{
    Configuration, ConfigurationErrors, ConfigurationIssue, ConfigurationResult, Secret,
};
pub use context::{ApplicationContext, ConstructionContext};
pub use descriptor::{
    CauldronDescriptor, CauldronImportDescriptor, DependencyDescriptor, ProviderConstructor,
    ProviderDescriptor, ProviderFuture, ProviderKind, ProviderVisibility,
};
#[doc(hidden)]
pub use descriptor::{LifecycleProviderConstructor, LifecycleProviderFuture, ProviderContribution};
pub use diagnostic::{
    Diagnostic, DiagnosticCode, Error, FURNACE001, FURNACE002, FURNACE003, FURNACE004, FURNACE005,
    FURNACE006, FURNACE007, FURNACE008, FURNACE009, FURNACE010, FURNACE011, FURNACE020, FURNACE030,
    Result, SourceLocation,
};
pub use graph::{
    ApplicationGraph, CauldronGraph, CauldronImportEdge, CauldronNode, ConstructionPlan,
    ConstructionStep, DependencyEdge, GraphAnalysis, ProviderNode, ProviderOrigin,
    ProviderOwnership, ProviderState,
};
#[doc(hidden)]
pub use graph::{
    AutoConfigurationInspectionSnapshot, CauldronImportInspectionSnapshot,
    CauldronInspectionSnapshot, DependencyInspectionSnapshot, GraphInspectionSnapshot,
    OwnedSourceLocation, ProviderInspectionSnapshot,
};
#[doc(hidden)]
pub use injector::injector_descriptor;
pub use injector::{InjectionDependencies, Injector};
pub use lifecycle::{
    LifecycleFuture, LifecycleHook, LifecycleManager, LifecycleResource, LifecycleState,
};
pub use registry::{ErasedProvider, ProviderRegistry};

pub use furnace_rs_core_macros::{Configuration, burner, cauldron, element, main, storage, test};

/// Implementation details used by furnace-rs procedural macro expansions.
#[doc(hidden)]
pub mod __private {
    use std::any::TypeId;

    pub use crate::configuration::parse as configuration;
    pub use crate::configuration::validate as configuration_validation;

    pub use crate::auto_configuration::{
        AutoConfigurationApplyContext, AutoConfigurationContext, AutoConfigurationContribution,
        AutoConfigurationDescriptor, AutoConfigurationEvaluation,
    };
    pub use crate::injector::InjectorMetadata;
    pub use crate::preflight::{PreflightContext, PreflightDescriptor, PreflightValidator};
    pub use inventory;

    /// Builds a rooted module graph for integration coverage and downstream framework crates.
    pub fn build_cauldron_graph<M: crate::Cauldron>() -> crate::Result<crate::CauldronGraph> {
        let cauldrons = crate::Catalog::cauldrons();
        crate::graph::build_cauldron_graph(TypeId::of::<M>(), &cauldrons)
    }
}
