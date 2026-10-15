//! Conditional attributes gate both callable methods and their policy metadata.
#![cfg(feature = "jwt")]
use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs_core::burner;
use furnace_rs_grpc::{GrpcApplicationScope, PassportPrincipal, grpc_service, guard, tonic};

struct Principal;
impl PassportPrincipal for Principal {
    fn has_role(&self, _: &str) -> bool {
        false
    }
    fn has_permission(&self, _: &str) -> bool {
        false
    }
}
#[guard(strategy = "conditional-metadata", principal = Principal)]
struct Protected;

#[burner]
struct NestedConditions;
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET, guard = Protected)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for NestedConditions {
    #[cfg_attr(all(), cfg_attr(all(), cfg(any())))]
    #[grpc_method(name = "Secret", guard = DisabledMissingGuard)]
    async fn secret(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        unreachable!()
    }

    #[cfg_attr(all(), allow(dead_code))]
    #[grpc_method(name = "Secret", public)]
    async fn secret(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Ok(tonic::Response::new(request.into_inner()))
    }
}

// A disabled implementation must not leave type, server or guard references.
#[grpc_service(server = MissingServer, descriptor = MISSING_DESCRIPTOR, guard = MissingGuard)]
#[cfg_attr(all(), cfg(any()))]
impl MissingTrait for MissingProvider {}

#[test]
fn nested_conditions_remove_disabled_override_and_keep_active_public_policy() {
    let scope =
        GrpcApplicationScope::for_focus(std::any::TypeId::of::<NestedConditions>()).unwrap();
    assert_eq!(scope.services()[0].methods().len(), 1);
    assert!(scope.services()[0].methods()[0].guard.is_none());
}

#[burner]
struct ConditionalPublic;
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET, guard = Protected)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for ConditionalPublic {
    #[cfg_attr(all(), cfg(any()))]
    #[grpc_method(name = "Secret", public)]
    async fn secret(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        unreachable!()
    }
    async fn secret(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Ok(tonic::Response::new(request.into_inner()))
    }
}

#[test]
fn cfg_attr_disabled_public_override_keeps_inherited_guard() {
    let scope =
        GrpcApplicationScope::for_focus(std::any::TypeId::of::<ConditionalPublic>()).unwrap();
    assert!(scope.services()[0].methods()[0].guard.is_some());
}
