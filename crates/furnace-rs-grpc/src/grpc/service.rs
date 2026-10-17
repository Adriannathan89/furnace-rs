//! Typed gRPC service registration in explicit application cauldrons.
use super::GrpcServiceDescriptor;
use furnace_rs_core::__private::TransportProvider;

/// A managed implementation connected to a protobuf-generated Tonic server.
pub trait GrpcService: TransportProvider + Clone {
    /// Returns the implementation's immutable protobuf registration metadata.
    #[doc(hidden)]
    fn grpc_descriptor() -> &'static GrpcServiceDescriptor;
}
