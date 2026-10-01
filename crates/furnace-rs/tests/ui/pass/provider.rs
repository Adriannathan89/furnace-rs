//! Confirms synchronous and asynchronous provider functions compile.

use furnace_rs::core::{Config, Result};

struct AsyncDirect;

struct SyncFallible;

struct CoreFallible;

#[furnace_rs::element]
fn sync_value() -> String {
    "value".to_owned()
}

#[furnace_rs::element]
async fn async_value(config: Config) -> furnace_rs::core::Result<usize> {
    Ok(config.len())
}

#[furnace_rs::element]
async fn async_direct() -> AsyncDirect {
    AsyncDirect
}

#[furnace_rs::element]
fn sync_fallible() -> Result<SyncFallible> {
    Ok(SyncFallible)
}

#[furnace_rs::element]
fn core_fallible() -> furnace_rs_core::Result<CoreFallible> {
    Ok(CoreFallible)
}

fn __furnace_construct_sync_value() {}

fn main() {
    let _user_function: fn() = __furnace_construct_sync_value;
}
