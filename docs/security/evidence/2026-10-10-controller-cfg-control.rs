#[furnace_rs::controller]
struct Controller;
#[furnace_rs::controller]
impl Controller {
    #[furnace_rs::get("/")]
    fn route(&self) -> &'static str { "ok" }
}
fn main() {}
