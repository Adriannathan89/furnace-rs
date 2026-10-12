use furnace_rs::{burner, cauldron, Furnace, Logger};

#[burner]
struct Greeting { #[inject] logger: Logger }
impl Greeting {
    fn greet(&self) { self.logger.info("compile-time DI example is running"); }
}
#[cauldron(providers=[Greeting], exports=[Greeting])]
struct GreetingCauldron;
#[cauldron(root, imports=[GreetingCauldron])]
struct AppCauldron;

fn main() {}
