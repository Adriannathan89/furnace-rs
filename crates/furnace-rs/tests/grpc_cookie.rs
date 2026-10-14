//! Cookie policies remain valid for HTTP but are rejected for protobuf admission.
#![cfg(all(feature = "grpc", feature = "jwt", feature = "cookies"))]
use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs::core::{Furnace, Injector, burner, cauldron};
use furnace_rs::{ClaimsPrincipal, PassportPrincipal, grpc_service, guard, tonic};
#[derive(serde::Deserialize)]
struct Claims;
impl PassportPrincipal for Claims {
    fn has_role(&self, _: &str) -> bool {
        false
    }
    fn has_permission(&self, _: &str) -> bool {
        false
    }
}
#[guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>, source = cookie("session"))]
struct CookieGuard;
#[burner]
struct Service;
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET, guard = CookieGuard)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Service {
    async fn secret(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        panic!("cookie guard must fail before handlers")
    }
}
struct ConstructorSentinel;
impl Injector for ConstructorSentinel {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs::core::Result<Self> {
        panic!("invalid gRPC policy must fail before constructors")
    }
}
#[cauldron(root, providers = [ConstructorSentinel], grpc_services = [Service])]
struct App;
#[tokio::test]
async fn cookie_guard_rejected_during_preflight() {
    let mut builder = Furnace::builder();
    builder.root::<App>().unwrap();
    let error = match builder.build().await {
        Ok(_) => panic!("cookie policy accepted by gRPC"),
        Err(error) => error,
    };
    assert_eq!(error.code(), furnace_rs::grpc::FURNACE150);
}
