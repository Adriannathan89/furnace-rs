//! Typed gRPC service registration in explicit application cauldrons.
use super::GrpcServiceDescriptor;
use furnace_rs_core::{__private::TransportProvider, Cauldron, CauldronRegistration};

/// A managed implementation connected to a protobuf-generated Tonic server.
pub trait GrpcService: TransportProvider + Clone {
    /// Returns the implementation's immutable protobuf registration metadata.
    #[doc(hidden)]
    fn grpc_descriptor() -> &'static GrpcServiceDescriptor;
}

/// Adds gRPC services to a fluent cauldron registration.
pub trait GrpcCauldronExt: Sized {
    /// Registers one typed implementation and its protobuf service metadata.
    fn grpc_service<T: GrpcService>(self) -> Self;
}
impl<M: Cauldron> GrpcCauldronExt for CauldronRegistration<M> {
    fn grpc_service<T: GrpcService>(self) -> Self {
        self.transport_provider::<T>()
    }
}
