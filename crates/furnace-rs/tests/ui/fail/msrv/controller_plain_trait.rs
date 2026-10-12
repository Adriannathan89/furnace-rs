trait PlainRoute {
    fn index(&self);
}

#[furnace_rs::controller(routes = [PlainRoute])]
struct Controller;

impl PlainRoute for Controller {
    fn index(&self) {}
}

fn main() {}
