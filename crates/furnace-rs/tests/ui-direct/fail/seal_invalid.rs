use furnace_rs::controller;
#[controller]
struct Controller;
#[controller]
impl Controller {
    #[get]
    #[seal(unknown)]
    fn endpoint(&self) {}
}
fn main() {}
