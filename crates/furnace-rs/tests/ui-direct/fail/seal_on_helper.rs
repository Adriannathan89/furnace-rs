use furnace_rs::controller;
#[controller]
struct Controller;
#[controller]
impl Controller {

    #[seal(skip)]
    fn endpoint(&self) {}
}
fn main() {}
