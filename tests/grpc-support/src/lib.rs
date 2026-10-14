//! Generated transport contracts for framework integration tests.
/// Test protobuf contracts.
pub mod pb {
    tonic::include_proto!("furnace.test.v1");
}
/// Encoded file descriptors, including the unregistered Hidden service.
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("transport");
