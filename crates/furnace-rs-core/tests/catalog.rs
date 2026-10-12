//! Integration tests for deterministic descriptor discovery.

use std::any::TypeId;
use std::fs;
use std::process::Command;
use std::sync::Arc;

use furnace_rs_core::{
    Catalog, Cauldron, CauldronDescriptor, ConstructionContext, ErasedProvider, FURNACE001,
    FURNACE002, FURNACE003, Furnace, ProviderDescriptor, ProviderFuture, ProviderKind,
    ProviderVisibility, SourceLocation,
};

struct Alpha;
struct Duplicate;
struct DuplicateCauldron;
struct Missing;
struct MissingCauldron;
struct Zeta;

#[furnace_rs_core::cauldron]
struct ImportedCauldron;

impl furnace_rs_core::Cauldron for ImportedCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        furnace_rs_core::CauldronRegistration::new(self)
    }
}

#[furnace_rs_core::cauldron]
struct SecondImportedCauldron;

impl furnace_rs_core::Cauldron for SecondImportedCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        furnace_rs_core::CauldronRegistration::new(self)
    }
}

#[furnace_rs_core::cauldron]
struct AnnotatedCauldron;

impl furnace_rs_core::Cauldron for AnnotatedCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        self.import(SecondImportedCauldron).import(ImportedCauldron)
    }
}

impl Cauldron for DuplicateCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        furnace_rs_core::CauldronRegistration::new(self)
    }
}
impl Cauldron for MissingCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        furnace_rs_core::CauldronRegistration::new(self)
    }
}

fn alpha_type_id() -> TypeId {
    TypeId::of::<Alpha>()
}

fn zeta_type_id() -> TypeId {
    TypeId::of::<Zeta>()
}

fn duplicate_type_id() -> TypeId {
    TypeId::of::<Duplicate>()
}

fn duplicate_cauldron_type_id() -> TypeId {
    TypeId::of::<DuplicateCauldron>()
}

fn alpha_constructor<'a>(_: &'a ConstructionContext<'a>) -> ProviderFuture<'a> {
    Box::pin(async { Ok(Arc::new(Alpha) as ErasedProvider) })
}

fn duplicate_constructor<'a>(_: &'a ConstructionContext<'a>) -> ProviderFuture<'a> {
    Box::pin(async { Ok(Arc::new(Duplicate) as ErasedProvider) })
}

inventory::submit! {
    CauldronDescriptor::new("zeta::Cauldron", zeta_type_id, SourceLocation::new(file!(), line!(), column!()))
}

inventory::submit! {
    CauldronDescriptor::new("alpha::Cauldron", alpha_type_id, SourceLocation::new(file!(), line!(), column!()))
}

inventory::submit! {
    CauldronDescriptor::new(
        "duplicate::Cauldron",
        duplicate_cauldron_type_id,
        SourceLocation::new("duplicate_module.rs", 1, 1),
    )
}

inventory::submit! {
    CauldronDescriptor::new(
        "duplicate::Cauldron",
        duplicate_cauldron_type_id,
        SourceLocation::new("duplicate_module.rs", 1, 1),
    )
}

inventory::submit! {
    ProviderDescriptor::new(
        ProviderKind::Provider,
        "alpha::Provider",
        alpha_type_id,
        &[],
        ProviderVisibility::Private,
        SourceLocation::new(file!(), line!(), column!()),
        alpha_constructor,
    )
}

inventory::submit! {
    ProviderDescriptor::new(
        ProviderKind::Service,
        "duplicate::First",
        duplicate_type_id,
        &[],
        ProviderVisibility::Private,
        SourceLocation::new(file!(), line!(), column!()),
        duplicate_constructor,
    )
}

inventory::submit! {
    ProviderDescriptor::new(
        ProviderKind::Repository,
        "duplicate::Second",
        duplicate_type_id,
        &[],
        ProviderVisibility::Private,
        SourceLocation::new(file!(), line!(), column!()),
        duplicate_constructor,
    )
}

#[test]
fn modules_are_sorted_by_stable_name() {
    let names: Vec<_> = Catalog::cauldrons()
        .into_iter()
        .map(|item| item.type_name())
        .collect();

    assert_eq!(
        names,
        [
            "alpha::Cauldron",
            concat!(module_path!(), "::AnnotatedCauldron"),
            concat!(module_path!(), "::ImportedCauldron"),
            concat!(module_path!(), "::SecondImportedCauldron"),
            "duplicate::Cauldron",
            "duplicate::Cauldron",
            "zeta::Cauldron",
        ]
    );
}

#[test]
fn cauldron_for_selects_the_annotated_cauldron_descriptor() {
    fn assert_module<T: Cauldron>() {}

    assert_module::<AnnotatedCauldron>();

    let descriptor = Catalog::cauldron_for::<AnnotatedCauldron>()
        .expect("annotated module descriptor should be selected");

    assert_eq!(
        descriptor.type_name(),
        concat!(module_path!(), "::AnnotatedCauldron")
    );
    assert_eq!(descriptor.namespace(), Some(module_path!()));
    assert!(descriptor.registration().is_some());
    let graph = furnace_rs_core::__private::build_cauldron_graph::<AnnotatedCauldron>().unwrap();
    assert_eq!(graph.imports().len(), 2);
    assert_eq!(
        graph.imports()[0].imported(&graph).type_id(),
        TypeId::of::<SecondImportedCauldron>()
    );
    assert_eq!(
        graph.imports()[1].imported(&graph).type_id(),
        TypeId::of::<ImportedCauldron>()
    );
}

#[test]
fn module_macro_requires_imports_to_implement_module() {
    let consumer = tempfile::tempdir().expect("temporary consumer directory should exist");
    let source_dir = consumer.path().join("src");
    fs::create_dir(&source_dir).expect("temporary consumer source directory should exist");

    let core_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        consumer.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"module-import-bound\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nfurnace-rs-core = {{ path = {core_path:?} }}\n"
        ),
    )
    .expect("temporary consumer manifest should be written");
    fs::write(
        source_dir.join("main.rs"),
        "struct NotACauldron;\n#[furnace_rs_core::cauldron]\nstruct AppCauldron;\nimpl furnace_rs_core::Cauldron for AppCauldron { fn register(self) -> furnace_rs_core::CauldronRegistration<Self> { self.import(NotACauldron) } }\nfn main() {}\n",
    )
    .expect("temporary consumer source should be written");

    let output = Command::new(env!("CARGO"))
        .args(["check", "--offline", "--quiet", "--manifest-path"])
        .arg(consumer.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", consumer.path().join("target"))
        .output()
        .expect("temporary consumer should invoke cargo check");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "consumer unexpectedly compiled");
    assert!(
        stderr.contains("NotACauldron: Cauldron") && stderr.contains("trait bound"),
        "consumer failed for an unexpected reason:\n{stderr}"
    );
}

#[test]
fn cauldron_for_reports_repeated_exact_identities() {
    let Err(error) = Catalog::cauldron_for::<DuplicateCauldron>() else {
        panic!("duplicate module declarations should be rejected");
    };

    assert_eq!(error.code(), FURNACE001);
}

#[test]
fn cauldron_for_reports_missing_metadata() {
    let Err(error) = Catalog::cauldron_for::<MissingCauldron>() else {
        panic!("missing module metadata should be rejected");
    };

    assert_eq!(error.code(), FURNACE003);
}

#[test]
fn provider_for_selects_the_matching_descriptor() {
    let descriptor = Catalog::provider_for::<Alpha>().expect("alpha provider should be selected");

    assert_eq!(descriptor.type_name(), "alpha::Provider");
    assert_eq!(descriptor.type_id(), TypeId::of::<Alpha>());
}

#[test]
fn provider_for_reports_ambiguous_descriptors() {
    let Err(error) = Catalog::provider_for::<Duplicate>() else {
        panic!("duplicates should be rejected");
    };

    assert_eq!(error.code(), FURNACE002);
}

#[test]
fn provider_for_reports_a_missing_descriptor() {
    let Err(error) = Catalog::provider_for::<Missing>() else {
        panic!("missing providers should be rejected");
    };

    assert_eq!(error.code(), FURNACE003);
}

#[tokio::test]
async fn manual_construction_rejects_ambiguous_provider_outputs() {
    let mut builder = Furnace::builder();
    let Err(error) = builder.construct::<Duplicate>().await else {
        panic!("different providers for one output type must be ambiguous");
    };

    assert_eq!(error.code(), FURNACE002);
}
