use furnace_rs::{guard, PassportPrincipal};
#[derive(PassportPrincipal)]
struct Principal { #[roles] roles: Vec<String>, #[permissions] permissions: Vec<String> }
fn allowed(_: &Principal) -> bool { true }
#[guard(strategy = "custom", principal = Principal, roles(any = ["admin"]), permissions(all = ["read"]), predicates = [allowed])]
struct Policy;
#[guard(strategy = "custom", principal = Principal, source = cookie("session"))]
struct CookiePolicy;
fn main() {}

#[guard(strategy = "custom", principal = Principal)]
#[cfg_attr(all(), cfg(any()))]
struct DisabledPolicy;

#[guard(strategy = "custom", principal = Principal)]
#[cfg_attr(all(), derive(Clone))]
struct DerivedPolicy;

#[derive(serde::Deserialize, PassportPrincipal)]
struct Claims { subject: String }
#[guard(strategy = "jwt", principal = furnace_rs::ClaimsPrincipal<Claims>)]
struct JwtPolicy;
