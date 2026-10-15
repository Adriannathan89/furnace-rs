//! Defensive, socket-free comparison of conditional RPC admission policies.
#![cfg(feature = "jwt")]

use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs_core::{ConfigBuilder, Furnace, MapSource, burner, cauldron};
use furnace_rs_grpc::{
    GrpcApplicationScope, JwtClaims, JwtService, JwtSignOptions, JwtTokenKind, PassportContext,
    PassportPrincipal, PassportResult, PassportStrategy, grpc_service, guard, passport_strategy,
    tonic,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Claims {
    subject: u64,
}
struct Principal;
impl PassportPrincipal for Principal {
    fn has_role(&self, _: &str) -> bool {
        false
    }
    fn has_permission(&self, _: &str) -> bool {
        false
    }
}
#[burner]
struct Strategy;
#[passport_strategy(name = "conditional-policy-test")]
impl PassportStrategy for Strategy {
    type Claims = Claims;
    type Principal = Principal;
    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;
    async fn validate(
        &self,
        _: &PassportContext<'_>,
        _: &JwtClaims<Claims>,
    ) -> PassportResult<Principal> {
        Ok(Principal)
    }
}
#[guard(strategy = "conditional-policy-test", principal = Principal)]
struct Protected;
static GUARDED_CALLS: AtomicUsize = AtomicUsize::new(0);

#[burner]
struct Conditional;
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET, guard = Protected)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Conditional {
    #[cfg(any())]
    #[grpc_method(name = "Secret", public)]
    async fn secret(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        unreachable!()
    }

    #[cfg(not(any()))]
    async fn secret(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        GUARDED_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(tonic::Response::new(request.into_inner()))
    }
}

#[burner]
struct Public;
#[grpc_service(server = pb::hidden_server::HiddenServer, descriptor = FILE_DESCRIPTOR_SET, guard = Protected)]
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Public {
    #[grpc_method(name = "Secret", public)]
    async fn secret(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Ok(tonic::Response::new(request.into_inner()))
    }
}

#[cauldron(providers = [Strategy, JwtService], grpc_services = [Conditional])]
struct GuardedRoot;
#[cauldron(grpc_services = [Public])]
struct PublicRoot;

async fn application<M: furnace_rs_core::Cauldron>() -> Furnace {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "synthetic regression configuration",
            [(
                "passport.secret",
                "local-only-conditional-policy-regression-key",
            )],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<M>().unwrap();
    builder.build().await.unwrap()
}

#[tokio::test]
async fn policy_comparison() {
    let rounds: usize = std::env::var("FURNACE_GRPC_POLICY_ROUNDS")
        .map(|value| value.parse().expect("integer rounds"))
        .unwrap_or(2);
    assert!((1..=10_000).contains(&rounds));
    GUARDED_CALLS.store(0, Ordering::SeqCst);
    let guarded = application::<GuardedRoot>().await;
    let public = application::<PublicRoot>().await;
    let scope = GrpcApplicationScope::for_application(&guarded).unwrap();
    let guard_retained = scope.services()[0].methods()[0].guard.is_some();
    let token = guarded
        .context()
        .resolve::<JwtService>()
        .unwrap()
        .sign(
            &Claims { subject: 7 },
            JwtSignOptions::access(Duration::from_secs(60)),
        )
        .unwrap();
    let mut client = pb::hidden_client::HiddenClient::new(
        furnace_rs_grpc::build_grpc_routes(&guarded)
            .unwrap()
            .prepare(),
    );
    let mut public_client = pb::hidden_client::HiddenClient::new(
        furnace_rs_grpc::build_grpc_routes(&public)
            .unwrap()
            .prepare(),
    );
    let (mut anonymous_rejected, mut authenticated_accepted, mut public_accepted) = (0, 0, 0);
    for _ in 0..rounds {
        if client
            .secret(pb::Item { value: 42 })
            .await
            .is_err_and(|status| status.code() == tonic::Code::Unauthenticated)
        {
            anonymous_rejected += 1;
        }
        let mut request = tonic::Request::new(pb::Item { value: 42 });
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer {token}").parse().unwrap());
        if client
            .secret(request)
            .await
            .is_ok_and(|response| response.into_inner().value == 42)
        {
            authenticated_accepted += 1;
        }
        if public_client
            .secret(pb::Item { value: 42 })
            .await
            .is_ok_and(|response| response.into_inner().value == 42)
        {
            public_accepted += 1;
        }
    }
    let handler_calls = GUARDED_CALLS.load(Ordering::SeqCst);
    let result = serde_json::json!({
        "rounds": rounds,
        "guard_retained": guard_retained,
        "anonymous_rejected": anonymous_rejected,
        "authenticated_accepted": authenticated_accepted,
        "public_accepted": public_accepted,
        "guarded_handler_calls": handler_calls,
    });
    println!("FURNACE_GRPC_POLICY_RESULT {result}");
    assert!(guard_retained, "active RPC must retain its service guard");
    assert_eq!(anonymous_rejected, rounds, "anonymous RPC must be rejected");
    assert_eq!(authenticated_accepted, rounds);
    assert_eq!(
        public_accepted, rounds,
        "explicit public RPC must stay public"
    );
    assert_eq!(
        handler_calls, rounds,
        "only authenticated calls enter handler"
    );
}
