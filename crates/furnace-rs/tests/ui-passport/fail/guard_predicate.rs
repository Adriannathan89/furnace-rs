use furnace_rs::common::*;

struct UserPrincipal;

impl PassportPrincipal for UserPrincipal {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

fn wrong_predicate_argument(_: UserPrincipal) -> bool {
    true
}

fn wrong_predicate_return(_: &UserPrincipal) {}

#[guard(strategy = "jwt", principal = UserPrincipal, predicate = wrong_predicate_argument)]
struct ArgumentPolicy;
#[guard(strategy = "jwt", principal = UserPrincipal, predicate = wrong_predicate_return)]
struct ReturnPolicy;
fn main() {}
