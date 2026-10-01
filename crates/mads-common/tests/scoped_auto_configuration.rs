//! Rooted auto-configuration requirements follow the selected application scope.

#![cfg(all(feature = "http", feature = "jwt"))]
#![allow(missing_docs)]

use mads_common::{
    ClaimsPrincipal, MADS121,
    core::{AutoConfigurationReport, AutoConfigurationStatus, Config, Furnace, Mads, Result},
};

#[derive(serde::Deserialize)]
struct UnreachableClaims;

impl mads_common::PassportPrincipal for UnreachableClaims {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

mod public_http {
    #[mads_common::routes]
    pub trait PublicRoutes {
        #[mads_common::get("/public")]
        async fn public(&self) -> &'static str;
    }

    #[mads_common::controller(routes = [PublicRoutes])]
    pub struct PublicController;

    impl PublicRoutes for PublicController {
        async fn public(&self) -> &'static str {
            "public"
        }
    }

    #[mads_common::core::furnace]
    pub struct PublicHttpModule;

    impl mads_common::core::Furnace for PublicHttpModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.controller::<PublicController>()
        }
    }
}

mod guarded_http {
    use super::*;

    #[mads_common::routes]
    #[mads_common::guard(strategy = "jwt", principal = ClaimsPrincipal<UnreachableClaims>)]
    pub trait GuardedRoutes {
        #[mads_common::get("/guarded")]
        async fn guarded(&self) -> &'static str;
    }

    #[mads_common::controller(routes = [GuardedRoutes])]
    pub struct GuardedController;

    impl GuardedRoutes for GuardedController {
        async fn guarded(&self) -> &'static str {
            "guarded"
        }
    }

    #[mads_common::core::furnace]
    pub struct GuardedHttpModule;

    impl mads_common::core::Furnace for GuardedHttpModule {
        fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
            self.controller::<GuardedController>()
        }
    }
}

mod roots {
    pub(super) mod public {
        #[mads_common::core::furnace]
        pub struct PublicRoot;

        impl mads_common::core::Furnace for PublicRoot {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.import(super::super::public_http::PublicHttpModule)
            }
        }
    }

    pub(super) mod guarded {
        #[mads_common::core::furnace]
        pub struct GuardedRoot;

        impl mads_common::core::Furnace for GuardedRoot {
            fn register(self) -> mads_common::core::FurnaceRegistration<Self> {
                self.import(super::super::public_http::PublicHttpModule)
                    .import(super::super::guarded_http::GuardedHttpModule)
            }
        }
    }
}

async fn build_root<M: Furnace>(config: Config) -> Result<Mads> {
    let mut builder = Mads::builder_with_config(config);
    builder.root::<M>()?;
    builder.build().await
}

fn report<'a>(application: &'a Mads, identifier: &str) -> &'a AutoConfigurationReport {
    application
        .auto_configurations()
        .iter()
        .find(|report| report.identifier() == identifier)
        .expect("the official auto-configuration descriptor must be registered")
}

#[tokio::test]
async fn unreachable_guard_does_not_require_configuration() {
    let application = build_root::<roots::public::PublicRoot>(Config::empty())
        .await
        .unwrap();

    assert_eq!(
        report(&application, "mads.common.passport.jwt").status(),
        AutoConfigurationStatus::Skipped,
    );
}

#[tokio::test]
async fn reachable_guard_still_requires_jwt_configuration() {
    let error = match build_root::<roots::guarded::GuardedRoot>(Config::empty()).await {
        Ok(_) => panic!("a reachable JWT guard must require Passport configuration"),
        Err(error) => error,
    };

    assert_eq!(error.code(), MADS121);
}
