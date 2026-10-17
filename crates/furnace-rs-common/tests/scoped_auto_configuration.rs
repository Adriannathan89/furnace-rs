//! Rooted auto-configuration requirements follow the selected application scope.

#![cfg(all(feature = "http", feature = "jwt"))]
#![allow(missing_docs)]

use furnace_rs_common::{
    ClaimsPrincipal, FURNACE121,
    core::{
        __private::ModuleContract, AutoConfigurationReport, AutoConfigurationStatus, Config,
        Furnace, Result,
    },
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

    #[furnace_rs_common::controller]
    pub struct PublicController;

    impl ::furnace_rs_common::Sealable for PublicController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            ::furnace_rs_common::SealRegistration::new()
        }
    }

    #[furnace_rs_common::controller]
    impl PublicController {
        #[furnace_rs_common::get("/public")]
        async fn public(&self) -> &'static str {
            "public"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [PublicController])]
    pub struct PublicHttpCauldron;
}

mod guarded_http {
    use super::*;

    #[furnace_rs_common::controller]
    pub struct GuardedController;

    #[furnace_rs_common::guard(principal = ClaimsPrincipal < UnreachableClaims >, strategy = "jwt")]
    struct GuardedControllerGuard;

    impl ::furnace_rs_common::Sealable for GuardedController {
        fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
            Self::seal::<GuardedControllerGuard>()
        }
    }

    #[furnace_rs_common::controller]
    impl GuardedController {
        #[furnace_rs_common::get("/guarded")]
        async fn guarded(&self) -> &'static str {
            "guarded"
        }
    }

    #[furnace_rs_common::core::cauldron(controllers = [GuardedController])]
    pub struct GuardedHttpCauldron;
}

mod roots {
    pub(super) mod public {
        #[furnace_rs_common::core::cauldron(imports = [super::super::public_http::PublicHttpCauldron])]
        pub struct PublicRoot;
    }

    pub(super) mod guarded {
        #[furnace_rs_common::core::cauldron(imports = [super::super::public_http::PublicHttpCauldron, super::super::guarded_http::GuardedHttpCauldron])]
        pub struct GuardedRoot;
    }
}

async fn build_root<M: ModuleContract>(config: Config) -> Result<Furnace> {
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
