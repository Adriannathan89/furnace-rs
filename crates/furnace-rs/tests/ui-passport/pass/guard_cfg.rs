#[cfg(any())]
struct DisabledClaims;

#[cfg(any())]
fn disabled_predicate(_: &furnace_rs::ClaimsPrincipal<DisabledClaims>) -> bool {
    true
}

#[furnace_rs::controller]
struct ConditionallyGuardedRoutesController;

#[cfg(any())]
#[furnace_rs::guard(predicate = disabled_predicate, principal = furnace_rs::ClaimsPrincipal < DisabledClaims >, strategy = "jwt")]
struct ConditionallyGuardedRoutesControllerGuard;

impl ::furnace_rs::Sealable for ConditionallyGuardedRoutesController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        {
            let registration = furnace_rs::SealRegistration::new();
            #[cfg(any())]
            let registration = registration.seal::<ConditionallyGuardedRoutesControllerGuard>();
            registration
        }
    }
}

#[furnace_rs::controller]
impl ConditionallyGuardedRoutesController {
    #[cfg(any())]
    #[get("/disabled")]

    async fn disabled(&self) {
        unreachable!("metadata-only endpoint")
    }
}

fn main() {}
