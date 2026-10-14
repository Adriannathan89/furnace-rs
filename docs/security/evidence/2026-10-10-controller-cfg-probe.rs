#[furnace_rs::controller]
struct Controller {
    #[cfg(any())]
    disabled: MissingDependency,
    #[cfg_attr(all(), cfg(any()))]
    attribute_disabled: AnotherMissingDependency,
}
#[furnace_rs::controller]
impl Controller {
    #[furnace_rs::get("/")]
    fn route(&self) -> &'static str { "ok" }
}
fn main() {}
