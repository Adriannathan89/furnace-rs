//! Shared application runtime and gRPC composition.
pub use furnace_rs_common::runtime::*;

mod adapter;
mod server;

pub use adapter::RuntimeGrpcExt;

mod inspection;
