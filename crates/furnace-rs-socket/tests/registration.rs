//! Static registration and transport-selection behavior.
use furnace_rs_core::{Config, ConfigBuilder, MapSource};
use furnace_rs_socket::common::runtime::RuntimeSelection;
use furnace_rs_socket::{
    EndpointMode, ProcessingMode, SocketApplicationScope, SocketEndpointDescriptor,
    SocketHandlerDescriptor,
};
use std::any::TypeId;
fn selected(names: &[&str]) -> Config {
    ConfigBuilder::new()
        .source(
            MapSource::new("test", [] as [(&str, &str); 0])
                .with_string_array("furnace.transports", names.iter().copied()),
        )
        .build()
        .unwrap()
}
fn descriptor(
    path: &'static str,
    handlers: &'static [SocketHandlerDescriptor],
) -> SocketEndpointDescriptor {
    SocketEndpointDescriptor::new(
        TypeId::of::<u8>,
        "Gateway",
        path,
        EndpointMode::Events,
        ProcessingMode::Sequential,
        handlers,
    )
}
#[test]
fn selection_accepts_socket_and_preserves_http_default() {
    let s = RuntimeSelection::parse_registered(&selected(&["socket"]), &["socket"]).unwrap();
    assert!(s.socket && !s.http && !s.grpc);
    let s = RuntimeSelection::parse_registered(
        &selected(&["http", "grpc", "socket"]),
        &["http", "grpc", "socket"],
    )
    .unwrap();
    assert!(s.http && s.grpc && s.socket);
    assert!(
        RuntimeSelection::parse_registered(&Config::empty(), &["http", "socket"])
            .unwrap()
            .http
    );
    for names in [&["socket", "socket"][..], &["socket"][..], &[][..]] {
        assert!(RuntimeSelection::parse_registered(&selected(names), &["http"]).is_err());
    }
}
#[test]
fn static_validation_rejects_duplicates_and_reserved_events() {
    let a = descriptor("/chat", &[]);
    let b = descriptor("/chat", &[]);
    assert!(SocketApplicationScope::validate_descriptors(&[&a, &b]).is_err());
    for handlers in [
        &[SocketHandlerDescriptor {
            name: "furnace.join",
            kind: "event",
        }][..],
        &[
            SocketHandlerDescriptor {
                name: "chat",
                kind: "event",
            },
            SocketHandlerDescriptor {
                name: "chat",
                kind: "event",
            },
        ][..],
    ] {
        let d = descriptor("/chat", handlers);
        assert!(SocketApplicationScope::validate_descriptors(&[&d]).is_err());
    }
    assert!(SocketApplicationScope::validate_descriptors(&[&descriptor("/chat", &[])]).is_ok());
}
struct Selected;
struct Unrelated;
impl furnace_rs_core::Injector for Selected {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        panic!("static scope must not construct")
    }
}
impl furnace_rs_core::Injector for Unrelated {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        panic!("unrelated")
    }
}
impl furnace_rs_core::__private::TransportProvider for Selected {
    const TRANSPORT: &'static str = "socket";
}
impl furnace_rs_core::__private::TransportProvider for Unrelated {
    const TRANSPORT: &'static str = "socket";
}
static SELECTED: SocketEndpointDescriptor = SocketEndpointDescriptor::new(
    TypeId::of::<Selected>,
    "Selected",
    "/same",
    EndpointMode::Events,
    ProcessingMode::Sequential,
    &[],
);
static UNRELATED: SocketEndpointDescriptor = SocketEndpointDescriptor::new(
    TypeId::of::<Unrelated>,
    "Unrelated",
    "/same",
    EndpointMode::Events,
    ProcessingMode::Sequential,
    &[],
);
furnace_rs_core::__private::inventory::submit! {furnace_rs_socket::SocketEndpointRegistration {descriptor: &SELECTED}}
furnace_rs_core::__private::inventory::submit! {furnace_rs_socket::SocketEndpointRegistration {descriptor: &UNRELATED}}
#[furnace_rs_core::cauldron]
struct App;
impl furnace_rs_core::Cauldron for App {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        furnace_rs_core::CauldronRegistration::new(self).transport_provider::<Selected>()
    }
}
#[test]
fn scope_only_includes_reachable_providers_without_construction() {
    let graph = furnace_rs_core::__private::build_cauldron_graph::<App>().unwrap();
    let scope = SocketApplicationScope::for_cauldron_graph(Some(&graph)).unwrap();
    assert_eq!(scope.endpoints().len(), 1);
    assert_eq!(scope.endpoints()[0].implementation, "Selected");
}
