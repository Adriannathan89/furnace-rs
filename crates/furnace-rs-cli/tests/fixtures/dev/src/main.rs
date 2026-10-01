use std::{
    fs::OpenOptions,
    io::Write,
};

use furnace::prelude::*;

#[furnace::routes]
trait HealthRoutes {
    #[furnace::get("/health")]
    async fn health(&self) -> &'static str;
}

#[furnace::controller(routes = [HealthRoutes])]
struct HealthController;

impl HealthRoutes for HealthController {
    async fn health(&self) -> &'static str {
        "healthy"
    }
}

#[furnace::cauldron]
struct AppCauldron;

impl furnace::core::Cauldron for AppCauldron {
    fn register(self) -> furnace::core::CauldronRegistration<Self> {
        self.controller::<HealthController>()
    }
}


#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    let path = std::env::var("FURNACE_TEST_START_LOG")
        .expect("dev-loop test should provide a start log path");
    let arguments = std::env::args().skip(1).collect::<Vec<_>>().join("|");
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("start log should be writable");
    writeln!(log, "{}|{arguments}", std::process::id()).expect("start log entry should write");
    Furnace::burn::<AppCauldron>().await
}
