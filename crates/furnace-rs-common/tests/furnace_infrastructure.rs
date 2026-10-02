//! Official global furnaces participate only through reachable imports.
#![cfg(feature = "logger")]
use furnace_rs_common::{Logger, LoggerCauldron};
use furnace_rs_core::{Cauldron, CauldronRegistration, Furnace};
#[furnace_rs_core::burner]
struct Consumer {
    _logger: Logger,
}
#[furnace_rs_core::cauldron]
struct Feature;
impl Cauldron for Feature {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<Consumer>()
    }
}
#[furnace_rs_core::cauldron]
struct App;
impl Cauldron for App {
    fn register(self) -> CauldronRegistration<Self> {
        self.import(LoggerCauldron).import(Feature)
    }
}
#[test]
fn logger_global_requires_reachable_import() {
    let mut builder = Furnace::builder();
    builder.root::<App>().unwrap();
    assert!(builder.analyze().is_valid());
    let mut builder = Furnace::builder();
    builder.root::<Feature>().unwrap();
    assert!(!builder.analyze().is_valid());
}
