//! Passport role, permission, and predicate enforcement order.

#![cfg(all(feature = "http", feature = "jwt"))]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::{
    body::{Body, to_bytes},
    http::{
        Request, StatusCode,
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
    },
};
use furnace_rs_common::{
    JwtClaims, JwtService, JwtSignOptions, JwtTokenKind, PassportContext, PassportPrincipal,
    PassportResult, PassportStrategy, build_router, controller,
    core::{Config, ConfigBuilder, Furnace, MapSource},
    passport_strategy,
};
use tower::ServiceExt;

static EVENTS: AtomicUsize = AtomicUsize::new(0);
static HANDLER_CALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct PolicyClaims {
    roles: Vec<String>,
    permissions: Vec<String>,
    predicate: bool,
}

struct PolicyPrincipal(PolicyClaims);

impl PassportPrincipal for PolicyPrincipal {
    fn has_role(&self, role: &str) -> bool {
        record(match role {
            "member" => 2,
            "verified" => 6,
            "operator" => 8,
            "admin" => 9,
            _ => unreachable!("the test routes declare known roles"),
        });
        self.0.roles.iter().any(|candidate| candidate == role)
    }

    fn has_permission(&self, permission: &str) -> bool {
        record(match permission {
            "profile:read" => 3,
            "profile:write" => 7,
            "dash:read" => 4,
            "dash:write" => 5,
            _ => unreachable!("the test routes declare known permissions"),
        });
        self.0
            .permissions
            .iter()
            .any(|candidate| candidate == permission)
    }
}

#[furnace_rs_core::burner]
struct PolicyStrategy;

#[passport_strategy(name = "jwt")]
impl PassportStrategy for PolicyStrategy {
    type Claims = PolicyClaims;
    type Principal = PolicyPrincipal;

    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _context: &PassportContext<'_>,
        claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        record(1);
        Ok(PolicyPrincipal(claims.custom.clone()))
    }
}

fn permits_profile(principal: &PolicyPrincipal) -> bool {
    record(4);
    principal.0.predicate
}

#[controller]
struct PolicyController;

#[furnace_rs_common::guard(permissions (all = ["profile:read" , "profile:write"]), predicate = permits_profile, principal = PolicyPrincipal, roles (all = ["member" , "verified"]), strategy = "jwt")]
struct PolicyControllerGuard;

impl ::furnace_rs_common::Sealable for PolicyController {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        Self::seal::<PolicyControllerGuard>()
    }
}

#[furnace_rs_common::controller]
impl PolicyController {
    #[furnace_rs_common::get("/policy/all")]
    async fn all(&self) -> &'static str {
        record(5);
        HANDLER_CALLS.fetch_add(1, Ordering::SeqCst);
        "all"
    }
}

#[furnace_rs_common::controller]
struct PolicyControllerPolicy1;

#[furnace_rs_common::guard(permissions (any = ["dash:read" , "dash:write"]), principal = PolicyPrincipal, roles (any = ["operator" , "admin"]), strategy = "jwt")]
struct PolicyControllerPolicy1Guard;

impl ::furnace_rs_common::Sealable for PolicyControllerPolicy1 {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        Self::seal::<PolicyControllerPolicy1Guard>()
    }
}

#[furnace_rs_common::controller]
impl PolicyControllerPolicy1 {
    #[furnace_rs_common::get("/policy/any")]
    async fn any(&self) -> &'static str {
        HANDLER_CALLS.fetch_add(1, Ordering::SeqCst);
        "any"
    }
}

fn config() -> Config {
    ConfigBuilder::new()
        .source(MapSource::new(
            "furnace.toml",
            [("passport.secret", "01234567890123456789012345678901")],
        ))
        .build()
        .unwrap()
}

fn record(event: usize) {
    EVENTS
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            Some(current * 10 + event)
        })
        .unwrap();
}

async fn request(router: &axum::Router, path: &str, token: String) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn assert_forbidden(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(response.headers().get(WWW_AUTHENTICATE).is_none());
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .as_ref(),
        b"{\"error\":{\"code\":\"forbidden\",\"message\":\"access was denied\"}}"
    );
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 0);
}

fn claims(roles: &[&str], permissions: &[&str], predicate: bool) -> PolicyClaims {
    PolicyClaims {
        roles: roles.iter().map(|role| (*role).to_owned()).collect(),
        permissions: permissions
            .iter()
            .map(|permission| (*permission).to_owned())
            .collect(),
        predicate,
    }
}

#[tokio::test]
async fn policies_run_roles_then_permissions_then_predicates_and_short_circuit_failures() {
    let application = Furnace::builder_with_config(config())
        .build()
        .await
        .unwrap();
    let jwt = application.context().resolve::<JwtService>().unwrap();
    let router = build_router(&application).unwrap();

    HANDLER_CALLS.store(0, Ordering::SeqCst);
    EVENTS.store(0, Ordering::SeqCst);
    assert_forbidden(
        request(
            &router,
            "/policy/all",
            jwt.sign(
                claims(&["member"], &["profile:read", "profile:write"], true),
                JwtSignOptions::access(Duration::from_secs(60)),
            )
            .unwrap(),
        )
        .await,
    )
    .await;
    assert_eq!(EVENTS.load(Ordering::SeqCst), 126);

    HANDLER_CALLS.store(0, Ordering::SeqCst);
    EVENTS.store(0, Ordering::SeqCst);
    assert_forbidden(
        request(
            &router,
            "/policy/all",
            jwt.sign(
                claims(&["member", "verified"], &["profile:read"], true),
                JwtSignOptions::access(Duration::from_secs(60)),
            )
            .unwrap(),
        )
        .await,
    )
    .await;
    assert_eq!(EVENTS.load(Ordering::SeqCst), 12637);

    HANDLER_CALLS.store(0, Ordering::SeqCst);
    EVENTS.store(0, Ordering::SeqCst);
    assert_forbidden(
        request(
            &router,
            "/policy/all",
            jwt.sign(
                claims(
                    &["member", "verified"],
                    &["profile:read", "profile:write"],
                    false,
                ),
                JwtSignOptions::access(Duration::from_secs(60)),
            )
            .unwrap(),
        )
        .await,
    )
    .await;
    assert_eq!(EVENTS.load(Ordering::SeqCst), 126374);

    HANDLER_CALLS.store(0, Ordering::SeqCst);
    EVENTS.store(0, Ordering::SeqCst);
    let response = request(
        &router,
        "/policy/all",
        jwt.sign(
            claims(
                &["member", "verified"],
                &["profile:read", "profile:write"],
                true,
            ),
            JwtSignOptions::access(Duration::from_secs(60)),
        )
        .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(EVENTS.load(Ordering::SeqCst), 1263745);
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 1);

    HANDLER_CALLS.store(0, Ordering::SeqCst);
    assert_forbidden(
        request(
            &router,
            "/policy/any",
            jwt.sign(
                claims(&["member"], &["dash:read"], true),
                JwtSignOptions::access(Duration::from_secs(60)),
            )
            .unwrap(),
        )
        .await,
    )
    .await;

    let response = request(
        &router,
        "/policy/any",
        jwt.sign(
            claims(&["admin"], &["dash:write"], true),
            JwtSignOptions::access(Duration::from_secs(60)),
        )
        .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 1);
}
