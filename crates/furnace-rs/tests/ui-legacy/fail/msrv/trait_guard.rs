struct Principal;
impl furnace_rs::PassportPrincipal for Principal { fn has_role(&self, _: &str) -> bool { false } fn has_permission(&self, _: &str) -> bool { false } }
#[furnace_rs::routes]
#[furnace_rs::guard(strategy = "custom", principal = Principal)]
trait Routes { #[furnace_rs::get("/")] async fn endpoint(&self); }
fn main() {}
