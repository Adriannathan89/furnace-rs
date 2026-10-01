//! Confirms controller dependency constraints point at the declared field type.

// Keep the Clone suggestion on a two-digit line so stable and Rust 1.94
// render its help gutter identically.
//
//
//
//
//
struct NotClone;

#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::get("/")]
    async fn index(&self);
}

#[furnace_rs::controller(routes = [Routes])]
struct Controller {
    dependency: NotClone,
}

impl Routes for Controller {
    async fn index(&self) {}
}

fn main() {}
