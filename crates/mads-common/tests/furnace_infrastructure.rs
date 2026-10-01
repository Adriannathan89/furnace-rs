//! Official global furnaces participate only through reachable imports.
#![cfg(feature = "logger")]
use mads_common::{Logger, LoggerModule};
use mads_core::{Furnace, FurnaceRegistration, Mads};
#[mads_core::burner]
struct Consumer {
    _logger: Logger,
}
#[mads_core::furnace]
struct Feature;
impl Furnace for Feature {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<Consumer>()
    }
}
#[mads_core::furnace]
struct App;
impl Furnace for App {
    fn register(self) -> FurnaceRegistration<Self> {
        self.import(LoggerModule).import(Feature)
    }
}
#[test]
fn logger_global_requires_reachable_import() {
    let mut builder = Mads::builder();
    builder.root::<App>().unwrap();
    assert!(builder.analyze().is_valid());
    let mut builder = Mads::builder();
    builder.root::<Feature>().unwrap();
    assert!(!builder.analyze().is_valid());
}
