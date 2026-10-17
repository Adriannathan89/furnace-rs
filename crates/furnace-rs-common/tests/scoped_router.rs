//! Rooted HTTP route selection tests.

#![cfg(feature = "http")]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use furnace_rs_common::build_router;
use furnace_rs_common::core::{FURNACE030, Furnace};
use tower::ServiceExt;

mod users {

    #[furnace_rs_common::controller]
    pub struct UserController;

    impl ::furnace_rs_common::Sealable for UserController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            ::furnace_rs_common::SealRegistration::new()
        }
    }

    #[furnace_rs_common::controller]
    impl UserController {
        #[get("/users")]
        async fn users(&self) -> &'static str {
            "users"
        }
        #[get("/shared")]
        async fn shared(&self) -> &'static str {
            "shared"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [UserController])]
    pub struct UserHttpCauldron;
}

mod admin {

    #[furnace_rs_common::controller]
    pub struct AdminController;

    impl ::furnace_rs_common::Sealable for AdminController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            ::furnace_rs_common::SealRegistration::new()
        }
    }

    #[furnace_rs_common::controller]
    impl AdminController {
        #[get("/admin")]
        async fn admin(&self) -> &'static str {
            "admin"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [AdminController])]
    pub struct AdminHttpCauldron;
}

mod duplicate_one {

    #[furnace_rs_common::controller]
    pub struct DuplicateOneController;

    impl ::furnace_rs_common::Sealable for DuplicateOneController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            ::furnace_rs_common::SealRegistration::new()
        }
    }

    #[furnace_rs_common::controller]
    impl DuplicateOneController {
        #[get("/conflict")]
        async fn conflict(&self) -> &'static str {
            "one"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [DuplicateOneController])]
    pub struct DuplicateOneHttpCauldron;
}

mod duplicate_two {

    #[furnace_rs_common::controller]
    pub struct DuplicateTwoController;

    impl ::furnace_rs_common::Sealable for DuplicateTwoController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            ::furnace_rs_common::SealRegistration::new()
        }
    }

    #[furnace_rs_common::controller]
    impl DuplicateTwoController {
        #[get("/conflict")]
        async fn conflict(&self) -> &'static str {
            "two"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [DuplicateTwoController])]
    pub struct DuplicateTwoHttpCauldron;
}

mod nested_controller_scope {

    pub mod reachable {

        #[furnace_rs_common::controller]
        pub struct ReachableController;

        impl ::furnace_rs_common::Sealable for ReachableController {
            fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
                ::furnace_rs_common::SealRegistration::new()
            }
        }

        #[furnace_rs_common::controller]
        impl ReachableController {
            #[get("/nested-reachable-controller")]
            async fn reachable(&self) -> &'static str {
                "reachable"
            }
        }

        #[furnace_rs_common::core::cauldron(controllers = [ReachableController])]
        pub struct ReachableCauldron;
    }

    pub mod unimported {

        #[furnace_rs_common::controller]
        pub struct UnreachableController;

        impl ::furnace_rs_common::Sealable for UnreachableController {
            fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
                ::furnace_rs_common::SealRegistration::new()
            }
        }

        #[furnace_rs_common::controller]
        impl UnreachableController {
            #[get("/nested-unreachable-controller")]
            async fn unreachable(&self) -> &'static str {
                "unreachable"
            }
        }

        #[furnace_rs_common::core::cauldron(controllers = [UnreachableController])]
        pub struct UnreachableCauldron;
    }

    #[furnace_rs_common::core::cauldron(imports = [reachable::ReachableCauldron])]
    pub struct ParentApplication;
}

mod nested_route_scope {

    pub mod unimported {

        #[furnace_rs_common::core::cauldron]
        pub struct UnreachableContractCauldron;
    }

    pub mod reachable {

        #[furnace_rs_common::controller]
        pub struct ReachableController;

        impl ::furnace_rs_common::Sealable for ReachableController {
            fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
                ::furnace_rs_common::SealRegistration::new()
            }
        }

        #[furnace_rs_common::controller]
        impl ReachableController {
            #[get("/nested-reachable-contract")]
            async fn reachable(&self) -> &'static str {
                "reachable"
            }
            #[get("/nested-unreachable-contract")]
            async fn unreachable(&self) -> &'static str {
                "unreachable"
            }
        }

        #[furnace_rs_common::core::cauldron(controllers = [ReachableController])]
        pub struct ReachableCauldron;
    }

    #[furnace_rs_common::core::cauldron(imports = [reachable::ReachableCauldron])]
    pub struct ParentApplication;
}

mod applications {
    pub(super) mod users {
        #[furnace_rs_common::core::cauldron(imports = [super::super::users::UserHttpCauldron])]
        pub struct UsersApplication;
    }

    pub(super) mod users_and_admin {
        #[furnace_rs_common::core::cauldron(imports = [super::super::users::UserHttpCauldron, super::super::admin::AdminHttpCauldron])]
        pub struct UsersAndAdminApplication;
    }

    pub(super) mod conflicting {
        #[furnace_rs_common::core::cauldron(imports = [super::super::duplicate_one::DuplicateOneHttpCauldron, super::super::duplicate_two::DuplicateTwoHttpCauldron])]
        pub struct ConflictingApplication;
    }
}

async fn request_status(router: furnace_rs_common::axum::Router, path: &str) -> StatusCode {
    router
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn rooted_router_installs_only_controllers_owned_by_reachable_modules() {
    let mut builder = Furnace::builder();
    builder
        .root::<applications::users::UsersApplication>()
        .unwrap();
    let application = builder.build().await.unwrap();
    let router = build_router(&application).unwrap();

    assert_eq!(
        request_status(router.clone(), "/users").await,
        StatusCode::OK
    );
    assert_eq!(
        request_status(router, "/admin").await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn rooted_router_installs_routes_from_every_reachable_http_module() {
    let mut builder = Furnace::builder();
    builder
        .root::<applications::users_and_admin::UsersAndAdminApplication>()
        .unwrap();
    let application = builder.build().await.unwrap();
    let router = build_router(&application).unwrap();

    assert_eq!(
        request_status(router.clone(), "/users").await,
        StatusCode::OK
    );
    assert_eq!(request_status(router, "/admin").await, StatusCode::OK);
}

#[tokio::test]
async fn rooted_router_inherits_unowned_route_contracts_from_selected_controllers() {
    let mut builder = Furnace::builder();
    builder
        .root::<applications::users::UsersApplication>()
        .unwrap();
    let application = builder.build().await.unwrap();
    let router = build_router(&application).unwrap();

    assert_eq!(request_status(router, "/shared").await, StatusCode::OK);
}

#[tokio::test]
async fn rooted_router_excludes_unimported_child_controllers_under_a_reachable_parent_namespace() {
    let mut builder = Furnace::builder();
    builder
        .root::<nested_controller_scope::ParentApplication>()
        .unwrap();
    let application = builder.build().await.unwrap();
    let router = build_router(&application).unwrap();

    assert_eq!(
        request_status(router.clone(), "/nested-reachable-controller").await,
        StatusCode::OK
    );
    assert_eq!(
        request_status(router, "/nested-unreachable-controller").await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn registered_controller_keeps_contracts_from_other_rust_namespaces() {
    let mut builder = Furnace::builder();
    builder
        .root::<nested_route_scope::ParentApplication>()
        .unwrap();
    let application = builder.build().await.unwrap();
    let router = build_router(&application).unwrap();

    assert_eq!(
        request_status(router.clone(), "/nested-reachable-contract").await,
        StatusCode::OK
    );
    assert_eq!(
        request_status(router, "/nested-unreachable-contract").await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn rooted_router_rejects_conflicting_routes_within_selected_modules() {
    let mut builder = Furnace::builder();
    builder
        .root::<applications::conflicting::ConflictingApplication>()
        .unwrap();
    let error = builder
        .build()
        .await
        .err()
        .expect("selected route conflicts must fail before construction");
    assert_eq!(error.code(), FURNACE030);
}

#[tokio::test]
async fn rootless_router_retains_complete_route_catalog_validation() {
    let error = Furnace::builder()
        .build()
        .await
        .err()
        .expect("rootless builds validate every route before construction");
    assert_eq!(error.code(), FURNACE030);
}
