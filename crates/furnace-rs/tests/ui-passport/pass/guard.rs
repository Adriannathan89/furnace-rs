use furnace_rs::common::*;

struct UserPrincipal;

impl PassportPrincipal for UserPrincipal {
    fn has_role(&self, _role: &str) -> bool {
        true
    }

    fn has_permission(&self, _permission: &str) -> bool {
        true
    }
}

fn owns_profile(_: &UserPrincipal) -> bool {
    true
}

fn may_read(_: &UserPrincipal) -> bool {
    true
}

#[derive(serde::Deserialize)]
struct UserClaims;

impl PassportPrincipal for UserClaims {
    fn has_role(&self, _role: &str) -> bool {
        true
    }

    fn has_permission(&self, _permission: &str) -> bool {
        true
    }
}

#[furnace_rs::controller]
struct UserRoutesController;

#[furnace_rs::guard(permissions (any = ["profile:read"]), predicate = owns_profile, principal = UserPrincipal, roles (all = ["member"]), source = bearer, strategy = "jwt-refresh")]
struct UserRoutesControllerGuard;

impl ::furnace_rs::Sealable for UserRoutesController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        Self::seal::<UserRoutesControllerGuard>()
    }
}

#[furnace_rs::controller]
impl UserRoutesController {
    #[get("/users/profile")]

    async fn profile(&self) {
        unreachable!("metadata-only endpoint")
    }
}

#[furnace_rs::controller]
struct UserRoutesController1;

impl ::furnace_rs::Sealable for UserRoutesController1 {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller]
impl UserRoutesController1 {
    #[post("/users/login")]

    async fn login(&self) {
        unreachable!("metadata-only endpoint")
    }
}

#[furnace_rs::controller]
struct MethodOnlyRoutesController;

#[furnace_rs::guard(predicates = [owns_profile , may_read], principal = UserPrincipal, strategy = "jwt")]
struct MethodOnlyRoutesControllerGuard;

impl ::furnace_rs::Sealable for MethodOnlyRoutesController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        Self::seal::<MethodOnlyRoutesControllerGuard>()
    }
}

#[furnace_rs::controller]
impl MethodOnlyRoutesController {
    #[get("/method-only")]

    async fn method_only(&self) {
        unreachable!("metadata-only endpoint")
    }
}

#[furnace_rs::controller]
struct BuiltinJwtRoutesController;

#[furnace_rs::guard(principal = ClaimsPrincipal < UserClaims >, strategy = "jwt")]
struct BuiltinJwtRoutesControllerGuard;

impl ::furnace_rs::Sealable for BuiltinJwtRoutesController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        Self::seal::<BuiltinJwtRoutesControllerGuard>()
    }
}

#[furnace_rs::controller]
impl BuiltinJwtRoutesController {
    #[get("/builtin")]

    async fn builtin(&self) {
        unreachable!("metadata-only endpoint")
    }
}

fn main() {}
