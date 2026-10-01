//! Integration tests for invalid controller route declarations.

use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};

use furnace_rs::common::__private::RouterBuildContext;
use furnace_rs::common::{ControllerEndpointDescriptor, HttpMethod, RouteCatalog, RouteDescriptor};
use furnace_rs::core::{Furnace, Result, SourceLocation};

static REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);

struct FirstManualController;
struct SecondManualController;
struct CountedManualController;

fn first_manual_type_id() -> TypeId {
    TypeId::of::<FirstManualController>()
}

fn second_manual_type_id() -> TypeId {
    TypeId::of::<SecondManualController>()
}

fn counted_manual_type_id() -> TypeId {
    TypeId::of::<CountedManualController>()
}

fn no_op_registrar(
    router: furnace_rs::common::axum::Router,
    _: &RouterBuildContext<'_>,
    _: &mut furnace_rs::common::__private::ValidatedRouteIter<'_>,
) -> Result<furnace_rs::common::axum::Router> {
    Ok(router)
}

fn counted_registrar(
    router: furnace_rs::common::axum::Router,
    _: &RouterBuildContext<'_>,
    _: &mut furnace_rs::common::__private::ValidatedRouteIter<'_>,
) -> Result<furnace_rs::common::axum::Router> {
    REGISTRATIONS.fetch_add(1, Ordering::SeqCst);
    Ok(router)
}

const FIRST_MANUAL_ROUTE: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "/users",
    "/{id}",
    "/users/{id}",
    "by_id",
    SourceLocation::new("tests/first_controller.rs", 12, 3),
);
const SECOND_MANUAL_ROUTE: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "/users",
    "/{user_id}",
    "/users/{user_id}",
    "by_user_id",
    SourceLocation::new("tests/second_controller.rs", 21, 7),
);
const COUNTED_MANUAL_ROUTE: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/counted",
    "/counted",
    "counted",
    SourceLocation::new("tests/counted_controller.rs", 3, 1),
);
const FIRST_MANUAL_CONTRACTS: &[RouteDescriptor] = &[FIRST_MANUAL_ROUTE];
const SECOND_MANUAL_CONTRACTS: &[RouteDescriptor] = &[SECOND_MANUAL_ROUTE];
const COUNTED_MANUAL_CONTRACTS: &[RouteDescriptor] = &[COUNTED_MANUAL_ROUTE];

furnace_rs::core::__private::inventory::submit! {
    furnace_rs::common::ControllerDescriptor::new("test::CountedManualController", counted_manual_type_id, SourceLocation::new(file!(), line!(), column!()), furnace_rs::common::SealDefinition::default)
}

furnace_rs::core::__private::inventory::submit! {
    ControllerEndpointDescriptor::new("aaa::CountedManualController", counted_manual_type_id, SourceLocation::new(file!(), line!(), column!()),
        COUNTED_MANUAL_CONTRACTS,
        counted_registrar,
    )
}

#[furnace_rs::controller]
struct DuplicateRouteController;

impl ::furnace_rs::Sealable for DuplicateRouteController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller]
impl DuplicateRouteController {
    #[furnace_rs::get("/duplicate")]
    async fn first(&self) {}
    #[furnace_rs::get("/duplicate")]
    async fn second(&self) {}
}

#[tokio::test]
async fn router_validation_rejects_conflicts_before_any_registration() {
    REGISTRATIONS.store(0, Ordering::SeqCst);
    let error = Furnace::builder()
        .build()
        .await
        .err()
        .expect("conflicting routes must fail before construction or registration");

    assert_eq!(error.code(), furnace_rs::core::FURNACE030);
    assert!(error.to_string().contains("GET /duplicate"));
    assert_eq!(REGISTRATIONS.load(Ordering::SeqCst), 0);
}

#[furnace_rs::controller]
struct EquivalentParameterRouteController;

impl ::furnace_rs::Sealable for EquivalentParameterRouteController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller]
impl EquivalentParameterRouteController {
    #[furnace_rs::get("/users/{id}")]
    async fn by_id(&self) {}
    #[furnace_rs::get("/users/{user_id}")]
    async fn by_user_id(&self) {}
}

#[tokio::test]
async fn controller_construction_remains_independent_of_http_validation() {
    let mut builder = Furnace::builder();
    builder
        .construct::<EquivalentParameterRouteController>()
        .await
        .expect("metadata-only controller construction must succeed");

    let error = RouteCatalog::validate().unwrap_err();
    assert_eq!(error.code(), furnace_rs::core::FURNACE030);
    assert!(error.to_string().contains("GET /duplicate"));
}

#[test]
fn cross_controller_dynamic_conflicts_report_both_declarations() {
    let first = ControllerEndpointDescriptor::new(
        "test::FirstManualController",
        first_manual_type_id,
        SourceLocation::new(file!(), line!(), column!()),
        FIRST_MANUAL_CONTRACTS,
        no_op_registrar,
    );
    let second = ControllerEndpointDescriptor::new(
        "test::SecondManualController",
        second_manual_type_id,
        SourceLocation::new(file!(), line!(), column!()),
        SECOND_MANUAL_CONTRACTS,
        no_op_registrar,
    );

    let error = furnace_rs::common::__private::validate_descriptors(&[&first, &second])
        .expect_err("equivalent dynamic routes across controllers must conflict");

    assert_eq!(error.code(), furnace_rs::core::FURNACE030);
    let rendered = error.to_string();
    assert!(rendered.contains("tests/first_controller.rs:12:3"));
    assert!(rendered.contains("tests/second_controller.rs:21:7"));
}
