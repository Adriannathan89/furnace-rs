//! Rooted HTTP route selection tests.

#![cfg(feature = "http")]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use furnace_rs_common::core::{FURNACE030, Furnace};
use furnace_rs_common::{build_router, controller, routes};
use tower::ServiceExt;

mod users {
    use super::shared_contracts::SharedRoutes;
    use super::*;

    #[routes]
    pub trait UserRoutes {
        #[get("/users")]
        async fn users(&self) -> &'static str;
    }

    #[controller(routes = [UserRoutes, SharedRoutes])]
    pub struct UserController;

    impl UserRoutes for UserController {
        async fn users(&self) -> &'static str {
            "users"
        }
    }

    impl SharedRoutes for UserController {
        async fn shared(&self) -> &'static str {
            "shared"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct UserHttpCauldron;

    impl furnace_rs_common::core::Cauldron for UserHttpCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<UserController>()
        }
    }
}

mod shared_contracts {
    use super::*;

    #[routes]
    pub trait SharedRoutes {
        #[get("/shared")]
        async fn shared(&self) -> &'static str;
    }
}

mod admin {
    use super::*;

    #[routes]
    pub trait AdminRoutes {
        #[get("/admin")]
        async fn admin(&self) -> &'static str;
    }

    #[controller(routes = [AdminRoutes])]
    pub struct AdminController;

    impl AdminRoutes for AdminController {
        async fn admin(&self) -> &'static str {
            "admin"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct AdminHttpCauldron;

    impl furnace_rs_common::core::Cauldron for AdminHttpCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<AdminController>()
        }
    }
}

mod duplicate_one {
    use super::*;

    #[routes]
    pub trait DuplicateOneRoutes {
        #[get("/conflict")]
        async fn conflict(&self) -> &'static str;
    }

    #[controller(routes = [DuplicateOneRoutes])]
    pub struct DuplicateOneController;

    impl DuplicateOneRoutes for DuplicateOneController {
        async fn conflict(&self) -> &'static str {
            "one"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct DuplicateOneHttpCauldron;

    impl furnace_rs_common::core::Cauldron for DuplicateOneHttpCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<DuplicateOneController>()
        }
    }
}

mod duplicate_two {
    use super::*;

    #[routes]
    pub trait DuplicateTwoRoutes {
        #[get("/conflict")]
        async fn conflict(&self) -> &'static str;
    }

    #[controller(routes = [DuplicateTwoRoutes])]
    pub struct DuplicateTwoController;

    impl DuplicateTwoRoutes for DuplicateTwoController {
        async fn conflict(&self) -> &'static str {
            "two"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct DuplicateTwoHttpCauldron;

    impl furnace_rs_common::core::Cauldron for DuplicateTwoHttpCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<DuplicateTwoController>()
        }
    }
}

mod nested_controller_scope {
    use super::*;

    pub mod reachable {
        use super::*;

        #[routes]
        pub trait ReachableRoutes {
            #[get("/nested-reachable-controller")]
            async fn reachable(&self) -> &'static str;
        }

        #[controller(routes = [ReachableRoutes])]
        pub struct ReachableController;

        impl ReachableRoutes for ReachableController {
            async fn reachable(&self) -> &'static str {
                "reachable"
            }
        }

        #[furnace_rs_common::core::cauldron]
        pub struct ReachableCauldron;

        impl furnace_rs_common::core::Cauldron for ReachableCauldron {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.controller::<ReachableController>()
            }
        }
    }

    pub mod unimported {
        use super::*;

        #[routes]
        pub trait UnreachableRoutes {
            #[get("/nested-unreachable-controller")]
            async fn unreachable(&self) -> &'static str;
        }

        #[controller(routes = [UnreachableRoutes])]
        pub struct UnreachableController;

        impl UnreachableRoutes for UnreachableController {
            async fn unreachable(&self) -> &'static str {
                "unreachable"
            }
        }

        #[furnace_rs_common::core::cauldron]
        pub struct UnreachableCauldron;

        impl furnace_rs_common::core::Cauldron for UnreachableCauldron {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.controller::<UnreachableController>()
            }
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct ParentApplication;

    impl furnace_rs_common::core::Cauldron for ParentApplication {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.import(reachable::ReachableCauldron)
        }
    }
}

mod nested_route_scope {
    use super::*;

    pub mod unimported {
        use super::*;

        #[routes]
        pub trait UnreachableContract {
            #[get("/nested-unreachable-contract")]
            async fn unreachable(&self) -> &'static str;
        }

        #[furnace_rs_common::core::cauldron]
        pub struct UnreachableContractCauldron;

        impl furnace_rs_common::core::Cauldron for UnreachableContractCauldron {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                furnace_rs_common::core::CauldronRegistration::new(self)
            }
        }
    }

    pub mod reachable {
        use super::unimported::UnreachableContract;
        use super::*;

        #[routes]
        pub trait ReachableRoutes {
            #[get("/nested-reachable-contract")]
            async fn reachable(&self) -> &'static str;
        }

        #[controller(routes = [ReachableRoutes, UnreachableContract])]
        pub struct ReachableController;

        impl ReachableRoutes for ReachableController {
            async fn reachable(&self) -> &'static str {
                "reachable"
            }
        }

        impl UnreachableContract for ReachableController {
            async fn unreachable(&self) -> &'static str {
                "unreachable"
            }
        }

        #[furnace_rs_common::core::cauldron]
        pub struct ReachableCauldron;

        impl furnace_rs_common::core::Cauldron for ReachableCauldron {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.controller::<ReachableController>()
            }
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct ParentApplication;

    impl furnace_rs_common::core::Cauldron for ParentApplication {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.import(reachable::ReachableCauldron)
        }
    }
}

mod applications {
    pub(super) mod users {
        #[furnace_rs_common::core::cauldron]
        pub struct UsersApplication;

        impl furnace_rs_common::core::Cauldron for UsersApplication {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.import(super::super::users::UserHttpCauldron)
            }
        }
    }

    pub(super) mod users_and_admin {
        #[furnace_rs_common::core::cauldron]
        pub struct UsersAndAdminApplication;

        impl furnace_rs_common::core::Cauldron for UsersAndAdminApplication {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.import(super::super::users::UserHttpCauldron)
                    .import(super::super::admin::AdminHttpCauldron)
            }
        }
    }

    pub(super) mod conflicting {
        #[furnace_rs_common::core::cauldron]
        pub struct ConflictingApplication;

        impl furnace_rs_common::core::Cauldron for ConflictingApplication {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.import(super::super::duplicate_one::DuplicateOneHttpCauldron)
                    .import(super::super::duplicate_two::DuplicateTwoHttpCauldron)
            }
        }
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
    let application = builder.build().await.unwrap();

    let error = build_router(&application).expect_err("selected route conflicts must be rejected");
    assert_eq!(error.code(), FURNACE030);
}

#[tokio::test]
async fn rootless_router_retains_complete_route_catalog_validation() {
    let application = Furnace::builder().build().await.unwrap();

    let error = build_router(&application).expect_err("rootless builds validate every route");
    assert_eq!(error.code(), FURNACE030);
}
