//! `#[rate_limit]` on sealed and `#[seal(skip)]` endpoints, behind a real JWT guard.
#![allow(missing_docs)]
#![cfg(all(feature = "extra", feature = "jwt"))]

use furnace_rs::axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use furnace_rs::core::{ConfigBuilder, MapSource};
use furnace_rs::prelude::{ExtraCauldron, MemoryBackend, RedisExtra, *};
use std::time::Duration;
use tower::ServiceExt;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Claims {
    subject: String,
}

impl PassportPrincipal for Claims {
    fn has_role(&self, _: &str) -> bool {
        false
    }
    fn has_permission(&self, _: &str) -> bool {
        false
    }
}

#[guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>)]
struct Policy;

#[controller]
struct Limited;

impl Sealable for Limited {
    fn seals() -> SealRegistration<Self> {
        Self::seal::<Policy>()
    }
}

#[controller(route = "/g")]
impl Limited {
    #[get("/guarded")]
    #[rate_limit(limit = 1, per = 60)]
    fn guarded(&self) -> &'static str {
        "guarded"
    }

    #[get("/open")]
    #[seal(skip)]
    #[rate_limit(limit = 1, per = 60)]
    fn open(&self) -> &'static str {
        "open"
    }
}

#[cauldron(imports = [ExtraCauldron], controllers = [Limited])]
struct App;

async fn router_and_tokens() -> (furnace_rs::axum::Router, String, String) {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "fixture",
            [("passport.secret", "01234567890123456789012345678901")],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    builder
        .provide(RedisExtra::with_backend(MemoryBackend::default()))
        .unwrap();
    let app = builder.build().await.unwrap();
    let jwt = app.context().resolve::<JwtService>().unwrap();
    let token = |subject: &str| {
        jwt.sign(
            Claims {
                subject: subject.into(),
            },
            JwtSignOptions::access(Duration::from_secs(60)),
        )
        .unwrap()
    };
    let (a, b) = (token("a"), token("b"));
    (build_router(&app).unwrap(), a, b)
}

async fn get(
    router: &furnace_rs::axum::Router,
    path: &str,
    authorization: &str,
    cookie: &str,
) -> StatusCode {
    let request = Request::builder()
        .uri(path)
        .header(header::AUTHORIZATION, authorization)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();
    router.clone().oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn sealed_endpoint_limits_by_validated_token_not_by_other_headers() {
    let (router, a, b) = router_and_tokens().await;
    let path = "/g/guarded";
    assert_eq!(
        get(&router, path, &format!("Bearer {a}"), "x=1").await,
        StatusCode::OK
    );
    // Same token: a different Cookie and a differently-cased scheme are the same client.
    assert_eq!(
        get(&router, path, &format!("bEaReR  {a}"), "x=2").await,
        StatusCode::TOO_MANY_REQUESTS
    );
    // A different valid token is a different client.
    assert_eq!(
        get(&router, path, &format!("Bearer {b}"), "x=1").await,
        StatusCode::OK
    );
    // Rate limiting runs behind the guard: bad tokens are rejected, not counted.
    assert_eq!(
        get(&router, path, "Bearer nope", "x=1").await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn skipped_endpoint_ignores_credentials_for_identity() {
    let (router, _, _) = router_and_tokens().await;
    let path = "/g/open";
    assert_eq!(
        get(&router, path, "Bearer garbage-1", "x=1").await,
        StatusCode::OK
    );
    assert_eq!(
        get(&router, path, "Bearer garbage-2", "x=2").await,
        StatusCode::TOO_MANY_REQUESTS
    );
}
