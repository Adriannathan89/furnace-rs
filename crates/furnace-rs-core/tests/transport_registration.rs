//! A transport integration can tag providers without bringing transport types into core.
use furnace_rs_core::__private::TransportProvider;
use furnace_rs_core::{Furnace, burner, cauldron};
use std::any::TypeId;

#[burner]
struct TransportService;
impl TransportProvider for TransportService {
    const TRANSPORT: &'static str = "grpc";
}

#[burner]
struct OrdinaryService;

#[cauldron(root, providers = [OrdinaryService], grpc_services = [TransportService])]
struct DeclarativeRoot;

#[cauldron(providers = [OrdinaryService], grpc_services = [TransportService])]
struct UnmarkedRoot;

#[tokio::test]
async fn transport_membership_is_explicit_for_marked_and_unmarked_roots() {
    for graph in [
        furnace_rs_core::__private::build_cauldron_graph::<DeclarativeRoot>().unwrap(),
        furnace_rs_core::__private::build_cauldron_graph::<UnmarkedRoot>().unwrap(),
    ] {
        assert_eq!(
            graph
                .registered_transport_providers("grpc")
                .collect::<Vec<_>>(),
            [TypeId::of::<TransportService>()]
        );
        assert!(
            graph
                .registered_transport_providers("http")
                .next()
                .is_none()
        );
        assert!(!graph.is_controller(TypeId::of::<TransportService>()));
    }
    let mut builder = Furnace::builder();
    builder.root::<DeclarativeRoot>().unwrap();
    let app = builder.build().await.unwrap();
    assert!(app.context().resolve::<TransportService>().is_ok());
    assert!(app.context().resolve::<OrdinaryService>().is_ok());
}
