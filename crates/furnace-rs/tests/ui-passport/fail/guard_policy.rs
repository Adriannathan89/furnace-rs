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

#[guard(strategy = "jwt", principal = UserPrincipal, roles(any = []))]
struct UserPolicy;

fn main() {}
