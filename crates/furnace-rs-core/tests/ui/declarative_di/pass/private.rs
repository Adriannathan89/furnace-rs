#![deny(warnings)]
use furnace_rs_core::{burner, cauldron};
mod hidden {
    use super::*;
    #[burner] struct Private;
    #[burner] struct Consumer { private: Private }
    #[burner] pub struct Exported;
    #[cauldron(providers=[Private, Consumer, Exported], exports=[Exported])]
    struct PrivateModule;
    #[cauldron(imports=[PrivateModule])]
    pub struct Parent;
}
#[cauldron(root, imports=[hidden::Parent])]
struct App;
fn main() {}
