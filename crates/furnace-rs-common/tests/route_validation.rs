//! Integration tests for runtime validation of route metadata.

#![cfg(feature = "http")]

use std::any::TypeId;

use furnace_rs_common::core::{FURNACE030, Result, SourceLocation};
use furnace_rs_common::{ControllerEndpointDescriptor, HttpMethod, RouteDescriptor};

struct FirstController;
struct SecondController;

fn first_type_id() -> TypeId {
    TypeId::of::<FirstController>()
}

fn second_type_id() -> TypeId {
    TypeId::of::<SecondController>()
}

fn no_op_registrar(
    router: furnace_rs_common::axum::Router,
    _: &furnace_rs_common::__private::RouterBuildContext<'_>,
    _: &mut furnace_rs_common::__private::ValidatedRouteIter<'_>,
) -> Result<furnace_rs_common::axum::Router> {
    Ok(router)
}

const VALID_ROUTE: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "/users",
    "/{id}",
    "/users/{id}",
    "get_user",
    SourceLocation::new("tests/route_validation.rs", 10, 5),
);

const BAD_JOIN: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "/users",
    "/{id}",
    "/wrong/{id}",
    "get_user",
    SourceLocation::new("tests/route_validation.rs", 20, 5),
);

const BAD_PREFIX: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "/users/{id}",
    "/posts",
    "/users/{id}/posts",
    "list_posts",
    SourceLocation::new("tests/route_validation.rs", 30, 5),
);

const BAD_PATH: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users/",
    "/users/",
    "list_users",
    SourceLocation::new("tests/route_validation.rs", 40, 5),
);

const BAD_LOCATION: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users",
    "/users",
    "list_users",
    SourceLocation::new("", 0, 0),
);

const BAD_QUERY: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users?active=true",
    "/users?active=true",
    "list_users",
    SourceLocation::new("tests/route_validation.rs", 50, 5),
);
const BAD_CONTROL: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users\n",
    "/users\n",
    "list_users",
    SourceLocation::new("tests/route_validation.rs", 60, 5),
);
const BAD_ENCODING: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users%20all",
    "/users%20all",
    "list_users",
    SourceLocation::new("tests/route_validation.rs", 70, 5),
);
const BAD_EMPTY_SEGMENT: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users//all",
    "/users//all",
    "list_users",
    SourceLocation::new("tests/route_validation.rs", 80, 5),
);
const BAD_EMPTY_PARAMETER: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users/:",
    "/users/:",
    "list_users",
    SourceLocation::new("tests/route_validation.rs", 90, 5),
);
const BAD_PARAMETER_NAME: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users/:1id",
    "/users/:1id",
    "list_users",
    SourceLocation::new("tests/route_validation.rs", 100, 5),
);
const BAD_REPEATED_PARAMETER: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users/:id/:id",
    "/users/:id/:id",
    "list_users",
    SourceLocation::new("tests/route_validation.rs", 110, 5),
);
const BAD_EMBEDDED_PARAMETER: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "",
    "/users/id:tail",
    "/users/id:tail",
    "list_users",
    SourceLocation::new("tests/route_validation.rs", 120, 5),
);
const ROOT_ROUTE: RouteDescriptor = RouteDescriptor::new(
    HttpMethod::Get,
    "/",
    "/",
    "/",
    "root",
    SourceLocation::new("tests/route_validation.rs", 130, 5),
);

const FIRST_ROUTES: &[RouteDescriptor] = &[VALID_ROUTE];
const FIRST_CONTRACTS: &[RouteDescriptor] = FIRST_ROUTES;
const BAD_PREFIX_CONTRACTS: &[RouteDescriptor] = &[BAD_PREFIX];
const BAD_PATH_CONTRACTS: &[RouteDescriptor] = &[BAD_PATH];
const BAD_JOIN_CONTRACTS: &[RouteDescriptor] = &[BAD_JOIN];
const BAD_LOCATION_CONTRACTS: &[RouteDescriptor] = &[BAD_LOCATION];
const BAD_QUERY_CONTRACTS: &[RouteDescriptor] = &[BAD_QUERY];
const BAD_CONTROL_CONTRACTS: &[RouteDescriptor] = &[BAD_CONTROL];
const BAD_ENCODING_CONTRACTS: &[RouteDescriptor] = &[BAD_ENCODING];
const BAD_EMPTY_SEGMENT_CONTRACTS: &[RouteDescriptor] = &[BAD_EMPTY_SEGMENT];
const BAD_EMPTY_PARAMETER_CONTRACTS: &[RouteDescriptor] = &[BAD_EMPTY_PARAMETER];
const BAD_PARAMETER_NAME_CONTRACTS: &[RouteDescriptor] = &[BAD_PARAMETER_NAME];
const BAD_REPEATED_PARAMETER_CONTRACTS: &[RouteDescriptor] = &[BAD_REPEATED_PARAMETER];
const BAD_EMBEDDED_PARAMETER_CONTRACTS: &[RouteDescriptor] = &[BAD_EMBEDDED_PARAMETER];
const ROOT_CONTRACTS: &[RouteDescriptor] = &[ROOT_ROUTE];
fn controller(
    type_name: &'static str,
    type_id: fn() -> TypeId,
    contracts: &'static [RouteDescriptor],
) -> ControllerEndpointDescriptor {
    ControllerEndpointDescriptor::new(
        type_name,
        type_id,
        SourceLocation::new(file!(), line!(), column!()),
        contracts,
        no_op_registrar,
    )
}

fn assert_invalid(controllers: &[&ControllerEndpointDescriptor]) {
    let error = furnace_rs_common::__private::validate_descriptors(controllers)
        .expect_err("untrusted route metadata must be rejected");
    assert_eq!(error.code(), FURNACE030);
}

fn descriptor_with_path(path: &'static str) -> ControllerEndpointDescriptor {
    let routes = Box::leak(Box::new([RouteDescriptor::new(
        HttpMethod::Get,
        "",
        path,
        path,
        "invalid_route",
        SourceLocation::new("tests/route_validation.rs", 140, 5),
    )]));
    let contracts = routes;
    controller("test::InvalidPathController", first_type_id, contracts)
}

fn descriptor_with_prefix(
    prefix: &'static str,
    full_path: &'static str,
) -> ControllerEndpointDescriptor {
    let routes = Box::leak(Box::new([RouteDescriptor::new(
        HttpMethod::Get,
        prefix,
        "/users",
        full_path,
        "invalid_route",
        SourceLocation::new("tests/route_validation.rs", 150, 5),
    )]));
    let contracts = routes;
    controller("test::InvalidPrefixController", first_type_id, contracts)
}

#[test]
fn rejects_invalid_route_paths_and_source_coordinates() {
    for contracts in [
        BAD_PREFIX_CONTRACTS,
        BAD_PATH_CONTRACTS,
        BAD_JOIN_CONTRACTS,
        BAD_LOCATION_CONTRACTS,
    ] {
        let descriptor = controller("test::Controller", first_type_id, contracts);
        assert_invalid(&[&descriptor]);
    }

    for contracts in [
        BAD_QUERY_CONTRACTS,
        BAD_CONTROL_CONTRACTS,
        BAD_ENCODING_CONTRACTS,
        BAD_EMPTY_SEGMENT_CONTRACTS,
        BAD_EMPTY_PARAMETER_CONTRACTS,
        BAD_PARAMETER_NAME_CONTRACTS,
        BAD_REPEATED_PARAMETER_CONTRACTS,
        BAD_EMBEDDED_PARAMETER_CONTRACTS,
    ] {
        let descriptor = controller("test::Controller", first_type_id, contracts);
        assert_invalid(&[&descriptor]);
    }
}

#[test]
fn rejects_axum_reserved_and_malformed_capture_syntax() {
    for path in ["/*rest", "/{id", "/id}"] {
        let descriptor = descriptor_with_path(path);
        assert_invalid(&[&descriptor]);
    }

    for (prefix, full_path) in [
        ("/*rest", "/*rest/users"),
        ("/{id}", "/{id}/users"),
        ("/{id", "/{id/users"),
        ("/id}", "/id}/users"),
    ] {
        let descriptor = descriptor_with_prefix(prefix, full_path);
        assert_invalid(&[&descriptor]);
    }
}

#[test]
fn rejects_wildcards_before_axum_router_construction_can_panic() {
    let descriptor = descriptor_with_path("/*rest");
    let result = std::panic::catch_unwind(|| {
        furnace_rs_common::__private::validate_descriptors(&[&descriptor]).map(|_| {
            furnace_rs_common::axum::Router::<()>::new().route(
                "/*rest",
                furnace_rs_common::axum::routing::get(|| async { "unreachable" }),
            )
        })
    });

    let error = result
        .expect("invalid metadata must return an error before Axum can panic")
        .expect_err("Axum wildcard metadata must fail closed");
    assert_eq!(error.code(), FURNACE030);
}

#[test]
fn rejects_empty_and_duplicate_controller_identities() {
    let empty = controller("", first_type_id, FIRST_CONTRACTS);
    assert_invalid(&[&empty]);
    let first = controller("test::First", first_type_id, FIRST_CONTRACTS);
    let same_id = controller("test::Second", first_type_id, FIRST_CONTRACTS);
    assert_invalid(&[&first, &same_id]);
    let same_name = controller("test::First", second_type_id, FIRST_CONTRACTS);
    assert_invalid(&[&first, &same_name]);
}

#[test]
fn descriptors_retain_optional_declaration_namespaces() {
    let route = RouteDescriptor::new(
        HttpMethod::Get,
        "",
        "/health",
        "/health",
        "health",
        SourceLocation::new("routes.rs", 1, 1),
    );
    assert_eq!(route.namespace(), None);
    assert_eq!(
        route.with_namespace("delivery::health").namespace(),
        Some("delivery::health")
    );

    let controller = controller("delivery::HealthController", first_type_id, FIRST_CONTRACTS);
    assert_eq!(controller.namespace(), None);
    assert_eq!(
        controller.with_namespace("delivery::health").namespace(),
        Some("delivery::health")
    );
}

#[test]
fn validated_routes_translate_parameters_for_axum() {
    let descriptor = controller("test::Controller", first_type_id, FIRST_CONTRACTS);
    let controllers = furnace_rs_common::__private::validate_descriptors(&[&descriptor])
        .expect("valid metadata must produce validated routes");
    let mut routes = controllers[0].routes();

    assert_eq!(
        routes
            .next(HttpMethod::Get, "get_user")
            .expect("validated route must preserve method and handler"),
        Some("/users/{id}")
    );
    routes
        .finish()
        .expect("all validated routes must be consumed");
}

#[test]
fn validates_root_routes_and_reports_registrar_metadata_mismatches() {
    let descriptor = controller("test::RootController", first_type_id, ROOT_CONTRACTS);
    let controllers = furnace_rs_common::__private::validate_descriptors(&[&descriptor])
        .expect("root route metadata is valid");
    let mut routes = controllers[0].routes();
    assert_eq!(
        routes.next(HttpMethod::Get, "root").expect("root path"),
        Some("/")
    );
    routes.finish().expect("root route consumed");

    let descriptor = controller("test::Controller", first_type_id, FIRST_CONTRACTS);
    let controllers = furnace_rs_common::__private::validate_descriptors(&[&descriptor])
        .expect("valid metadata must produce validated routes");
    let mut routes = controllers[0].routes();
    assert!(routes.next(HttpMethod::Post, "get_user").is_err());

    let mut routes = controllers[0].routes();
    assert!(routes.next(HttpMethod::Get, "wrong_handler").is_err());

    let mut routes = controllers[0].routes();
    assert!(routes.finish().is_err());
}
