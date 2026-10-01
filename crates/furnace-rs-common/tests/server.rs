//! Public HTTP runtime error contract tests.

#![cfg(feature = "http")]

use std::error::Error as _;
use std::io;

use furnace_rs_common::core::{Diagnostic, Error, FURNACE020, Furnace};
use furnace_rs_common::{FurnaceBurnExt, HttpRuntimeError, serve_router};

mod standard_run {
    #[furnace_rs_common::routes]
    pub trait RoutedRoutes {
        #[furnace_rs_common::get("/standard-run-health")]
        async fn health(&self) -> &'static str;
    }

    #[furnace_rs_common::controller(routes = [RoutedRoutes])]
    pub struct RoutedController;

    impl RoutedRoutes for RoutedController {
        async fn health(&self) -> &'static str {
            "healthy"
        }
    }

    #[furnace_rs_common::core::cauldron]
    pub struct RoutedApp;

    impl furnace_rs_common::core::Cauldron for RoutedApp {
        fn register(self) -> furnace_rs_common::core::CauldronRegistration<Self> {
            self.controller::<RoutedController>()
        }
    }
}

fn core_error(message: &str) -> Error {
    Error::new(Diagnostic::new(FURNACE020, "runtime test failure", message))
}

#[test]
fn runtime_errors_preserve_structured_sources() {
    let bootstrap = HttpRuntimeError::Bootstrap(core_error("catalog invalid"));
    assert!(bootstrap.to_string().contains("HTTP bootstrap failed"));
    let source = bootstrap.source().unwrap().downcast_ref::<Error>().unwrap();
    assert_eq!(source.code(), FURNACE020);

    let bind = HttpRuntimeError::Bind(io::Error::new(io::ErrorKind::AddrInUse, "occupied"));
    assert!(bind.to_string().contains("HTTP listener bind failed"));
    assert_eq!(
        bind.source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::AddrInUse
    );

    let combined = HttpRuntimeError::OperationAndShutdown {
        operation: Box::new(bind),
        shutdown: core_error("cleanup failed"),
    };
    assert!(combined.to_string().contains("shutdown also failed"));
    assert!(matches!(combined.source(), Some(source) if source.is::<HttpRuntimeError>()));
}

#[tokio::test]
async fn serve_router_is_available_for_raw_native_routers() {
    let application = Furnace::builder().build().await.unwrap();
    let runtime = serve_router(
        application,
        furnace_rs_common::axum::Router::new(),
        "127.0.0.1:0",
    );

    drop(runtime);
}

#[test]
fn run_extension_is_available_for_root_cauldrons() {
    let runtime = Furnace::burn::<standard_run::RoutedApp>();

    drop(runtime);
}
