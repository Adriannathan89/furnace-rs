//! Registered protobuf services are scoped and validated before DI construction.
#![cfg(feature = "grpc")]
use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs_common::{grpc_service, tonic};
use furnace_rs_core::{Furnace, Injector, cauldron};
use std::sync::atomic::{AtomicUsize, Ordering};

static CONSTRUCTORS: AtomicUsize = AtomicUsize::new(0);
static INVALID_CONSTRUCTORS: AtomicUsize = AtomicUsize::new(0);

macro_rules! provider {
    ($name:ident, $counter:ident) => {
        #[derive(Clone)]
        struct $name;
        impl Injector for $name {
            type Dependencies = ();
            async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
                $counter.fetch_add(1, Ordering::SeqCst);
                Ok(Self)
            }
        }
    };
}
provider!(Selected, CONSTRUCTORS);
provider!(Duplicate, INVALID_CONSTRUCTORS);
provider!(UnknownMethod, INVALID_CONSTRUCTORS);
provider!(MissingMetadata, INVALID_CONSTRUCTORS);
impl furnace_rs_core::__private::TransportProvider for MissingMetadata {
    const TRANSPORT: &'static str = "grpc";
}

#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Selected {
    async fn secret(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
}
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Duplicate {
    async fn secret(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
}
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for UnknownMethod {
    #[grpc_method(name = "Absent", public)]
    async fn secret(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Err(tonic::Status::unimplemented("test"))
    }
}
#[cauldron(grpc_services = [Selected])]
struct SelectedRoot;
#[cauldron(grpc_services = [Selected, Duplicate])]
struct DuplicateRoot;
#[cauldron(grpc_services = [UnknownMethod])]
struct UnknownRoot;
#[cauldron(providers = [])]
struct EmptyRoot;
#[cauldron(grpc_services = [MissingMetadata])]
struct MissingRoot;

#[test]
fn grpc_registered_provider_without_service_metadata_rejected() {
    let graph = furnace_rs_core::__private::build_cauldron_graph::<MissingRoot>().unwrap();
    assert_eq!(
        furnace_rs_common::GrpcApplicationScope::for_cauldron_graph(Some(&graph))
            .unwrap_err()
            .code(),
        furnace_rs_common::FURNACE150
    );
}

#[tokio::test]
async fn only_reachable_registered_services_selected() {
    let mut builder = Furnace::builder();
    builder.root::<SelectedRoot>().unwrap();
    let application = builder.build().await.unwrap();
    let scope = furnace_rs_common::GrpcApplicationScope::for_application(&application).unwrap();
    assert_eq!(scope.services().len(), 1);
    assert_eq!(scope.services()[0].name(), "furnace.test.v1.Hidden");
    let mut builder = Furnace::builder();
    builder.root::<EmptyRoot>().unwrap();
    let empty = builder.build().await.unwrap();
    assert!(
        furnace_rs_common::GrpcApplicationScope::for_application(&empty)
            .unwrap()
            .services()
            .is_empty()
    );
}

#[tokio::test]
async fn duplicate_wire_service_and_unknown_method_fail_preconstruction() {
    for graph in [
        furnace_rs_core::__private::build_cauldron_graph::<DuplicateRoot>().unwrap(),
        furnace_rs_core::__private::build_cauldron_graph::<UnknownRoot>().unwrap(),
    ] {
        let error =
            furnace_rs_common::GrpcApplicationScope::for_cauldron_graph(Some(&graph)).unwrap_err();
        assert_eq!(error.code(), furnace_rs_common::FURNACE150);
    }
    let mut builder = Furnace::builder();
    builder.root::<DuplicateRoot>().unwrap();
    let analysis = builder.analyze();
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|d| d.code() == furnace_rs_common::FURNACE150)
    );
    assert!(builder.build().await.is_err());
    assert_eq!(INVALID_CONSTRUCTORS.load(Ordering::SeqCst), 0);
}
