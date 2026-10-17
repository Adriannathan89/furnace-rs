use furnace_rs_core::{cauldron, Furnace};
#[cauldron]
struct Empty;
#[cauldron()]
struct Parentheses;
#[cauldron(root, imports = [Empty, Parentheses])]
struct App;
fn main() {
    Furnace::builder().root::<App>().unwrap();
}
