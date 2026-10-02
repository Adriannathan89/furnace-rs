#[furnace_rs::routes]
trait Routes { #[furnace_rs::get("/")] async fn endpoint(&self); }
#[furnace_rs::controller(routes = [Routes])]
struct Controller;
impl Routes for Controller { async fn endpoint(&self) {} }
fn main() {}
