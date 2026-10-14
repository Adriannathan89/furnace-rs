//! Application-owned protobuf serving and consumption example.
pub mod pb {
    tonic::include_proto!("demo.v1");
}
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("transport");
pub mod app;
pub mod auth;
pub mod client;
pub mod service;
