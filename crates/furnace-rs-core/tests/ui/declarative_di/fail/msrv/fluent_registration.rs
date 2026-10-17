use furnace_rs_core::{burner, cauldron};
#[burner]
struct Service;
#[cauldron]
struct App;
fn main() {
    App.provide::<Service>();
}
