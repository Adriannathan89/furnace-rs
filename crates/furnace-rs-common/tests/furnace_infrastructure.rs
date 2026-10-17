//! Official global furnaces participate only through reachable imports.
#![cfg(feature = "logger")]
use furnace_rs_common::{Logger, LoggerCauldron};
use furnace_rs_core::Furnace;
#[furnace_rs_core::burner]
struct Consumer {
    #[inject]
    _logger: Logger,
}
#[furnace_rs_core::cauldron(providers = [Consumer])]
struct Feature;

#[furnace_rs_core::cauldron(imports = [LoggerCauldron, Feature])]
struct App;

#[test]
fn logger_global_requires_reachable_import() {
    let mut builder = Furnace::builder();
    builder.root::<App>().unwrap();
    assert!(builder.analyze().is_valid());
    let mut builder = Furnace::builder();
    builder.root::<Feature>().unwrap();
    assert!(!builder.analyze().is_valid());
}
