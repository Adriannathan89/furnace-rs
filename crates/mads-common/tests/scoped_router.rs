//! Rooted HTTP route selection tests.

#![cfg(feature = "http")]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mads_common::core::{MADS030, Mads};
use mads_common::{build_router, controller, routes};
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

    #[mads_common::core::furnace]
    pub struct UserHttpModule;

    impl mads_common::core::Furnace for UserHttpModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
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

    #[mads_common::core::furnace]
    pub struct AdminHttpModule;

    impl mads_common::core::Furnace for AdminHttpModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
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

    #[mads_common::core::furnace]
    pub struct DuplicateOneHttpModule;

    impl mads_common::core::Furnace for DuplicateOneHttpModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
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

    #[mads_common::core::furnace]
    pub struct DuplicateTwoHttpModule;

    impl mads_common::core::Furnace for DuplicateTwoHttpModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
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

        #[mads_common::core::furnace]
        pub struct ReachableModule;

        impl mads_common::core::Furnace for ReachableModule {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
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

        #[mads_common::core::furnace]
        pub struct UnreachableModule;

        impl mads_common::core::Furnace for UnreachableModule {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.controller::<UnreachableController>()
            }
        }
    }

    #[mads_common::core::furnace]
    pub struct ParentApplication;

    impl mads_common::core::Furnace for ParentApplication {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.import(reachable::ReachableModule)
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

        #[mads_common::core::furnace]
        pub struct UnreachableContractModule;

        impl mads_common::core::Furnace for UnreachableContractModule {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                mads_common::core::FurnaceRegistration::new(self)
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

        #[mads_common::core::furnace]
        pub struct ReachableModule;

        impl mads_common::core::Furnace for ReachableModule {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.controller::<ReachableController>()
            }
        }
    }

    #[mads_common::core::furnace]
    pub struct ParentApplication;

    impl mads_common::core::Furnace for ParentApplication {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.import(reachable::ReachableModule)
        }
    }
}

mod applications {
    pub(super) mod users {
        #[mads_common::core::furnace]
        pub struct UsersApplication;

        impl mads_common::core::Furnace for UsersApplication {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.import(super::super::users::UserHttpModule)
            }
        }
    }

    pub(super) mod users_and_admin {
        #[mads_common::core::furnace]
        pub struct UsersAndAdminApplication;

        impl mads_common::core::Furnace for UsersAndAdminApplication {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.import(super::super::users::UserHttpModule)
                    .import(super::super::admin::AdminHttpModule)
            }
        }
    }

    pub(super) mod conflicting {
        #[mads_common::core::furnace]
        pub struct ConflictingApplication;

        impl mads_common::core::Furnace for ConflictingApplication {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.import(super::super::duplicate_one::DuplicateOneHttpModule)
                    .import(super::super::duplicate_two::DuplicateTwoHttpModule)
            }
        }
    }
}

async fn request_status(router: mads_common::axum::Router, path: &str) -> StatusCode {
    router
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn rooted_router_installs_only_controllers_owned_by_reachable_modules() {
    let mut builder = Mads::builder();
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
    let mut builder = Mads::builder();
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
    let mut builder = Mads::builder();
    builder
        .root::<applications::users::UsersApplication>()
        .unwrap();
    let application = builder.build().await.unwrap();
    let router = build_router(&application).unwrap();

    assert_eq!(request_status(router, "/shared").await, StatusCode::OK);
}

#[tokio::test]
async fn rooted_router_excludes_unimported_child_controllers_under_a_reachable_parent_namespace() {
    let mut builder = Mads::builder();
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
    let mut builder = Mads::builder();
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
    let mut builder = Mads::builder();
    builder
        .root::<applications::conflicting::ConflictingApplication>()
        .unwrap();
    let application = builder.build().await.unwrap();

    let error = build_router(&application).expect_err("selected route conflicts must be rejected");
    assert_eq!(error.code(), MADS030);
}

#[tokio::test]
async fn rootless_router_retains_complete_route_catalog_validation() {
    let application = Mads::builder().build().await.unwrap();

    let error = build_router(&application).expect_err("rootless builds validate every route");
    assert_eq!(error.code(), MADS030);
}
