//! Regression contracts for transport-independent Passport authentication.

use std::{any::TypeId, sync::Arc};

use axum::http::{Extensions, HeaderMap, HeaderValue, Method, Uri};
use furnace_rs_core::{Furnace, SourceLocation};

use super::{
    Authenticated, ErasedAuthentication, GuardDescriptor, GuardPredicate, PassportContext,
    PassportPrincipal, PassportStrategyFuture, PolicyClause, PolicyMode, TokenSource,
    VerifiedToken,
};
use crate::{JwtAlgorithm, JwtClaims, JwtHeader, JwtTokenKind, RegisteredJwtClaims, VerifiedJwt};

struct Principal {
    reader: bool,
    write: bool,
    active: bool,
}

#[furnace_rs_core::cauldron(providers = [])]
struct SharedTestRoot;

impl PassportPrincipal for Principal {
    fn has_role(&self, role: &str) -> bool {
        role == "reader" && self.reader
    }

    fn has_permission(&self, permission: &str) -> bool {
        permission == "read" || (permission == "write" && self.write)
    }
}

fn authentication(reader: bool, write: bool, active: bool) -> ErasedAuthentication {
    ErasedAuthentication::new(
        Principal {
            reader,
            write,
            active,
        },
        VerifiedJwt {
            header: JwtHeader {
                algorithm: JwtAlgorithm::Hs256,
                key_id: None,
                token_type: Some("furnace_rs-access+jwt".into()),
            },
            claims: JwtClaims {
                registered: RegisteredJwtClaims {
                    issuer: None,
                    subject: None,
                    audiences: vec![],
                    expires_at: 2,
                    not_before: None,
                    issued_at: 1,
                    jwt_id: None,
                    token_kind: JwtTokenKind::Access,
                },
                custom: 42_u64,
            },
        },
    )
}

fn adapter<'a>(
    _: &'a furnace_rs_core::ApplicationContext,
    context: &'a PassportContext<'a>,
    token: &'a str,
) -> PassportStrategyFuture<'a> {
    Box::pin(async move {
        assert_eq!(token, "opaque-token");
        for header in ["authorization", "proxy-authorization", "cookie"] {
            assert!(context.headers().get(header).is_none());
        }
        assert_eq!(context.headers()["x-request-id"], "request-123");
        assert_eq!(context.method(), Method::POST);
        assert_eq!(context.uri().path(), "/furnace.test.v1.Transport/Unary");
        Ok(authentication(true, true, true))
    })
}

#[tokio::test]
async fn bearer_context_strips_credentials() {
    let mut builder = Furnace::builder();
    builder.root::<SharedTestRoot>().unwrap();
    let application = builder.build().await.unwrap();
    let mut headers = HeaderMap::new();
    for (name, value) in [
        ("authorization", "Bearer opaque-token"),
        ("proxy-authorization", "Bearer proxy-secret"),
        ("cookie", "access=secret"),
        ("x-request-id", "request-123"),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    super::authentication::authenticate_bearer(
        application.context(),
        adapter,
        &headers,
        &Method::POST,
        &Uri::from_static("/furnace.test.v1.Transport/Unary"),
        None,
    )
    .await
    .unwrap();
}

#[test]
fn shared_policy_keeps_and_clauses() {
    static PREDICATES: [GuardPredicate; 1] = [GuardPredicate::new(
        "active",
        Some(|auth| auth.principal_as::<Principal>().unwrap().active),
    )];
    let guard = GuardDescriptor::new(
        "ReaderWriter",
        "operation",
        "jwt",
        Some(TypeId::of::<Principal>),
        Some(std::any::type_name::<Principal>),
        TokenSource::Bearer,
        Some(PolicyClause::new(PolicyMode::Any, &["admin", "reader"])),
        Some(PolicyClause::new(PolicyMode::All, &["read", "write"])),
        &PREDICATES,
        SourceLocation::new(file!(), line!(), column!()),
        None,
    );
    for (reader, write, active, allowed) in [
        (true, true, true, true),
        (false, true, true, false),
        (true, false, true, false),
        (true, true, false, false),
    ] {
        assert_eq!(
            super::policy::authorizes(&guard, &authentication(reader, write, active)),
            allowed
        );
    }
}

#[test]
fn http_adapter_retains_typed_extensions() {
    let auth = authentication(true, true, true);
    let principal = auth.principal_as::<Principal>().unwrap();
    let verified = auth.verified_as::<u64>().unwrap();
    let mut extensions = Extensions::new();
    assert!(auth.install_extensions(&mut extensions));
    let installed = extensions.get::<Authenticated<Principal>>().unwrap();
    let token = extensions.get::<VerifiedToken<u64>>().unwrap();
    assert!(std::ptr::eq(&**installed, Arc::as_ptr(&principal)));
    assert!(std::ptr::eq(token.verified(), Arc::as_ptr(&verified)));
    assert_eq!(token.claims.custom, 42);
}
