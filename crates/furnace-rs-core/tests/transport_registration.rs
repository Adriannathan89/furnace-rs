//! A transport integration can tag providers without bringing transport types into core.
use furnace_rs_core::__private::TransportProvider;
use furnace_rs_core::{Cauldron, CauldronRegistration, Furnace, burner, cauldron};
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

#[cauldron]
struct FluentRoot;
impl Cauldron for FluentRoot {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<OrdinaryService>()
            .transport_provider::<TransportService>()
    }
}

#[tokio::test]
async fn transport_membership_is_explicit_for_both_registration_styles() {
    for graph in [
        furnace_rs_core::__private::build_cauldron_graph::<DeclarativeRoot>().unwrap(),
        furnace_rs_core::__private::build_cauldron_graph::<FluentRoot>().unwrap(),
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
