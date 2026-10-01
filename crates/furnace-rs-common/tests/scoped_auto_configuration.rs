//! Rooted auto-configuration requirements follow the selected application scope.

#![cfg(all(feature = "http", feature = "jwt"))]
#![allow(missing_docs)]

use furnace_rs_common::{
    ClaimsPrincipal, FURNACE121,
    core::{AutoConfigurationReport, AutoConfigurationStatus, Cauldron, Config, Furnace, Result},
};

#[derive(serde::Deserialize)]
struct UnreachableClaims;

impl furnace_rs_common::PassportPrincipal for UnreachableClaims {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

mod public_http {
    #[furnace_rs_common::routes]
    pub trait PublicRoutes {
        #[furnace_rs_common::get("/public")]
        async fn public(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [PublicRoutes])]
    pub struct PublicController;

    impl PublicRoutes for PublicController {
        async fn public(&self) -> &'static str {
            "public"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct PublicHttpCauldron;

    impl furnace_rs_common::core::Cauldron for PublicHttpCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<PublicController>()
        }
    }
}

mod guarded_http {
    use super::*;

    #[furnace_rs_common::routes]
    #[furnace_rs_common::guard(strategy = "jwt", principal = ClaimsPrincipal<UnreachableClaims>)]
    pub trait GuardedRoutes {
        #[furnace_rs_common::get("/guarded")]
        async fn guarded(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [GuardedRoutes])]
    pub struct GuardedController;

    impl GuardedRoutes for GuardedController {
        async fn guarded(&self) -> &'static str {
            "guarded"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct GuardedHttpCauldron;

    impl furnace_rs_common::core::Cauldron for GuardedHttpCauldron {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<GuardedController>()
        }
    }
}

mod roots {
    pub(super) mod public {
        #[furnace_rs_common::core::cauldron]
        pub struct PublicRoot;

        impl furnace_rs_common::core::Cauldron for PublicRoot {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.import(super::super::public_http::PublicHttpCauldron)
            }
        }
    }

    pub(super) mod guarded {
        #[furnace_rs_common::core::cauldron]
        pub struct GuardedRoot;

        impl furnace_rs_common::core::Cauldron for GuardedRoot {
            fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
                self.import(super::super::public_http::PublicHttpCauldron)
                    .import(super::super::guarded_http::GuardedHttpCauldron)
            }
        }
    }
}

async fn build_root<M: Cauldron>(config: Config) -> Result<Furnace> {
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<M>()?;
    builder.build().await
}

fn report<'a>(application: &'a Furnace, identifier: &str) -> &'a AutoConfigurationReport {
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
        report(&application, "furnace.common.passport.jwt").status(),
        AutoConfigurationStatus::Skipped,
    );
}

#[tokio::test]
async fn reachable_guard_still_requires_jwt_configuration() {
    let error = match build_root::<roots::guarded::GuardedRoot>(Config::empty()).await {
        Ok(_) => panic!("a reachable JWT guard must require Passport configuration"),
        Err(error) => error,
    };

    assert_eq!(error.code(), FURNACE121);
}
