//! Renamed consumer with nested global, private module and diamond imports.
#![deny(warnings)]
use infra::Parent as Infrastructure;
use runtime::{burner, cauldron};
use std::sync::Arc;
#[cauldron(imports=[Infrastructure])] struct Left;
#[cauldron(imports=[Infrastructure])] struct Right;
type NativeAlias = Arc<String>;
#[burner] struct Consumer { _connection: NativeAlias }
#[cauldron(root, imports=[Left, Right], providers=[Consumer])] struct App;
fn main() {}
