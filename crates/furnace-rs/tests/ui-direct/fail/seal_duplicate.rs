use furnace_rs::controller;
#[controller]
struct Controller;
#[controller]
impl Controller {
    #[get]
    #[seal(skip)]
    #[seal(skip)]
    fn endpoint(&self) {}
}
fn main() {}
