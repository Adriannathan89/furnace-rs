mod furnace {}
use ::furnace::prelude::*;
#[derive(PassportPrincipal)]
struct Principal { subject: String }
#[guard(strategy = "custom", principal = Principal)]
struct Policy;
#[controller]
struct Controller;
impl Sealable for Controller { fn seals() -> SealRegistration<Self> { Self::seal::<Policy>() } }
#[controller(route = "/user")]
impl Controller { #[get] fn index(&self) -> &'static str { "ok" } }
fn main() {}

#[controller]
struct PublicWithoutSeal;
#[controller(route = "/public-default")]
impl PublicWithoutSeal { #[get] fn index(&self) -> &'static str { "open" } }
