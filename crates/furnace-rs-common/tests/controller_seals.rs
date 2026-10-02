//! Every inherent endpoint uses its controller's static policy before dispatch.
#![cfg(all(feature = "http", feature = "jwt"))]
#![allow(missing_docs)]
use axum::{
    body::{Body, to_bytes},
    http::{
        Method, Request, StatusCode,
        header::{AUTHORIZATION, COOKIE},
    },
};
use furnace_rs_common::core::{
    Cauldron, CauldronRegistration, Config, ConfigBuilder, Furnace, MapSource, cauldron,
};
use furnace_rs_common::{
    Authenticated, ClaimsPrincipal, Json, JwtService, JwtSignOptions, PassportPrincipal, Path,
    SealRegistration, Sealable, VerifiedToken, build_router, controller, guard,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tower::ServiceExt;
static HANDLERS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Claims {
    subject: String,
    role: bool,
    permission: bool,
    active: bool,
}
impl PassportPrincipal for Claims {
    fn has_role(&self, role: &str) -> bool {
        role == "admin" && self.role
    }
    fn has_permission(&self, permission: &str) -> bool {
        permission == "users:write" && self.permission
    }
}
fn active(principal: &ClaimsPrincipal<Claims>) -> bool {
    principal.active
}
#[guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>, roles(all = ["admin"]), permissions(all = ["users:write"]), predicate = active)]
struct Policy;
#[controller]
struct Users;
impl Sealable for Users {
    fn seals() -> SealRegistration<Self> {
        Self::seal::<Policy>()
    }
}
#[controller(route = "/users")]
impl Users {
    #[get("/:id")]
    fn find(
        &self,
        Path(id): Path<u64>,
        principal: Authenticated<ClaimsPrincipal<Claims>>,
        token: VerifiedToken<Claims>,
    ) -> String {
        HANDLERS.fetch_add(1, Ordering::SeqCst);
        assert_eq!(principal.subject, token.claims.custom.subject);
        format!("{}:{id}", principal.subject)
    }
    #[get]
    fn list(&self) -> &'static str {
        HANDLERS.fetch_add(1, Ordering::SeqCst);
        "users"
    }
    #[post]
    async fn create(&self, Json(value): Json<String>) -> String {
        HANDLERS.fetch_add(1, Ordering::SeqCst);
        value
    }
    #[get("/mixed")]
    #[seal(skip)]
    fn public_mixed(&self) -> &'static str {
        "open"
    }
    #[post("/mixed")]
    fn private_mixed(&self) -> &'static str {
        "protected"
    }
    #[cfg(any())]
    #[get("/mixed")]
    #[seal(skip)]
    fn disabled(&self) -> &'static str {
        unreachable!()
    }
}
#[controller]
struct Public;
impl Sealable for Public {
    fn seals() -> SealRegistration<Self> {
        SealRegistration::new()
    }
}
#[controller(route = "/public")]
impl Public {
    #[get]
    fn index(&self) -> &'static str {
        "public"
    }
}
#[cfg(feature = "cookies")]
#[guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>, source = cookie("session"))]
struct CookiePolicy;
#[cfg(feature = "cookies")]
#[controller]
struct CookieController;
#[cfg(feature = "cookies")]
impl Sealable for CookieController {
    fn seals() -> SealRegistration<Self> {
        Self::seal::<CookiePolicy>()
    }
}
#[cfg(feature = "cookies")]
#[controller(route = "/cookie")]
impl CookieController {
    #[get]
    fn index(&self, principal: Authenticated<ClaimsPrincipal<Claims>>) -> String {
        principal.subject.clone()
    }
}
#[cauldron]
struct App;
impl Cauldron for App {
    fn register(self) -> CauldronRegistration<Self> {
        let registration = self.controller::<Users>().controller::<Public>();
        #[cfg(feature = "cookies")]
        let registration = registration.controller::<CookieController>();
        registration
    }
}
fn config() -> Config {
    ConfigBuilder::new()
        .source(MapSource::new(
            "fixture",
            [("passport.secret", "01234567890123456789012345678901")],
        ))
        .build()
        .unwrap()
}
async fn application() -> Furnace {
    let mut builder = Furnace::builder_with_config(config());
    builder.root::<App>().unwrap();
    builder.build().await.unwrap()
}
fn token(app: &Furnace, role: bool, permission: bool, active: bool) -> String {
    app.context()
        .resolve::<JwtService>()
        .unwrap()
        .sign(
            Claims {
                subject: "user".into(),
                role,
                permission,
                active,
            },
            JwtSignOptions::access(Duration::from_secs(60)),
        )
        .unwrap()
}
async fn request(
    router: &axum::Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from("\"created\""))
        .unwrap();
    if let Some(token) = bearer {
        request
            .headers_mut()
            .insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
    }
    if let Some(cookie) = cookie {
        request
            .headers_mut()
            .insert(COOKIE, cookie.parse().unwrap());
    }
    router.clone().oneshot(request).await.unwrap()
}
#[tokio::test]
async fn seal_protects_get_by_id_base_get_and_post_before_any_handler() {
    let _lock = TEST_LOCK.lock().await;
    HANDLERS.store(0, Ordering::SeqCst);
    let app = application().await;
    let router = build_router(&app).unwrap();
    let endpoints = [
        (Method::GET, "/users"),
        (Method::GET, "/users/42"),
        (Method::POST, "/users"),
    ];
    for (method, path) in &endpoints {
        for credential in [None, Some("invalid-secret-sentinel")] {
            let response = request(&router, method.clone(), path, credential, None).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(
                !String::from_utf8_lossy(
                    &to_bytes(response.into_body(), usize::MAX).await.unwrap()
                )
                .contains("invalid-secret-sentinel")
            );
            assert_eq!(HANDLERS.load(Ordering::SeqCst), 0);
        }
        for (role, permission, active) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            let denied = token(&app, role, permission, active);
            assert_eq!(
                request(&router, method.clone(), path, Some(&denied), None)
                    .await
                    .status(),
                StatusCode::FORBIDDEN
            );
            assert_eq!(HANDLERS.load(Ordering::SeqCst), 0);
        }
    }
    let allowed = token(&app, true, true, true);
    for (method, path, expected) in [
        (Method::GET, "/users", "users"),
        (Method::GET, "/users/42", "user:42"),
        (Method::POST, "/users", "created"),
    ] {
        let response = request(&router, method, path, Some(&allowed), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            expected
        );
    }
    assert_eq!(HANDLERS.load(Ordering::SeqCst), 3);
    assert_eq!(
        request(&router, Method::GET, "/public", None, None)
            .await
            .status(),
        StatusCode::OK
    );
}
#[cfg(feature = "cookies")]
#[tokio::test]
async fn cookie_seal_uses_its_declared_source_and_installs_the_typed_principal() {
    let _lock = TEST_LOCK.lock().await;
    let app = application().await;
    let router = build_router(&app).unwrap();
    let token = token(&app, true, true, true);
    assert_eq!(
        request(&router, Method::GET, "/cookie", Some(&token), None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &router,
            Method::GET,
            "/cookie",
            None,
            Some("session=invalid-secret-sentinel")
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = request(
        &router,
        Method::GET,
        "/cookie",
        None,
        Some(&format!("session={token}")),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        "user"
    );
}

#[tokio::test]
async fn focused_controller_uses_endpoint_bindings_for_every_route() {
    let _lock = TEST_LOCK.lock().await;
    let mut builder = Furnace::builder_with_config(config());
    builder.__test_focus::<Users>().unwrap();
    let app = builder.build().await.unwrap();
    let router = furnace_rs_common::__private::build_test_router_for::<Users>(&app).unwrap();
    let allowed = token(&app, true, true, true);
    assert_eq!(
        request(&router, Method::GET, "/users", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(&router, Method::GET, "/users/42", Some(&allowed), None)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(&router, Method::POST, "/users", Some(&allowed), None)
            .await
            .status(),
        StatusCode::OK
    );
}

mod scoped {
    use super::*;
    use furnace_rs_common::{
        JwtClaims, JwtTokenKind, PassportContext, PassportResult, PassportStrategy,
        passport_strategy,
    };
    static FIRST: AtomicUsize = AtomicUsize::new(0);
    static SECOND: AtomicUsize = AtomicUsize::new(0);
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct ScopedClaims {
        expected: u8,
    }
    pub struct Principal {
        label: &'static str,
    }
    impl PassportPrincipal for Principal {
        fn has_role(&self, _: &str) -> bool {
            false
        }
        fn has_permission(&self, _: &str) -> bool {
            false
        }
    }
    #[guard(strategy = "jwt", principal = Principal)]
    struct SharedPolicy;
    mod first {
        use super::*;
        #[furnace_rs_common::core::burner]
        pub struct Strategy;
        #[passport_strategy(name = "jwt")]
        impl PassportStrategy for Strategy {
            type Claims = ScopedClaims;
            type Principal = Principal;
            const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;
            async fn validate(
                &self,
                _: &PassportContext<'_>,
                claims: &JwtClaims<ScopedClaims>,
            ) -> PassportResult<Principal> {
                FIRST.fetch_add(1, Ordering::SeqCst);
                if claims.custom.expected != 1 {
                    return Err(furnace_rs_common::PassportError::reject());
                }
                Ok(Principal { label: "first" })
            }
        }
        #[controller]
        pub struct Controller;
        impl Sealable for Controller {
            fn seals() -> SealRegistration<Self> {
                Self::seal::<SharedPolicy>()
            }
        }
        #[controller(route = "/first")]
        impl Controller {
            #[get]
            fn get(&self, principal: Authenticated<Principal>) -> &'static str {
                principal.label
            }
            #[post]
            fn post(&self, principal: Authenticated<Principal>) -> &'static str {
                principal.label
            }
        }
        #[cauldron]
        pub struct Module;
        impl Cauldron for Module {
            fn register(self) -> CauldronRegistration<Self> {
                self.provide::<Strategy>()
                    .export::<Strategy>()
                    .controller::<Controller>()
            }
        }
    }
    mod second {
        use super::*;
        #[furnace_rs_common::core::burner]
        pub struct Strategy;
        #[passport_strategy(name = "jwt")]
        impl PassportStrategy for Strategy {
            type Claims = ScopedClaims;
            type Principal = Principal;
            const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;
            async fn validate(
                &self,
                _: &PassportContext<'_>,
                claims: &JwtClaims<ScopedClaims>,
            ) -> PassportResult<Principal> {
                SECOND.fetch_add(1, Ordering::SeqCst);
                if claims.custom.expected != 2 {
                    return Err(furnace_rs_common::PassportError::reject());
                }
                Ok(Principal { label: "second" })
            }
        }
        #[controller]
        pub struct Controller;
        impl Sealable for Controller {
            fn seals() -> SealRegistration<Self> {
                Self::seal::<SharedPolicy>()
            }
        }
        #[controller(route = "/second")]
        impl Controller {
            #[get]
            fn get(&self, principal: Authenticated<Principal>) -> &'static str {
                principal.label
            }
        }
        #[cauldron]
        pub struct Module;
        impl Cauldron for Module {
            fn register(self) -> CauldronRegistration<Self> {
                self.provide::<Strategy>().controller::<Controller>()
            }
        }
    }
    #[cauldron]
    struct Both;
    impl Cauldron for Both {
        fn register(self) -> CauldronRegistration<Self> {
            self.import(first::Module).import(second::Module)
        }
    }
    #[controller]
    struct DirectController;
    impl Sealable for DirectController {
        fn seals() -> SealRegistration<Self> {
            Self::seal::<SharedPolicy>()
        }
    }
    #[controller(route = "/direct")]
    impl DirectController {
        #[get]
        fn get(&self, principal: Authenticated<Principal>) -> &'static str {
            principal.label
        }
    }
    #[cauldron]
    struct DirectRoot;
    impl Cauldron for DirectRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<DirectController>().import(first::Module)
        }
    }
    #[cauldron]
    struct GlobalStrategy;
    impl Cauldron for GlobalStrategy {
        fn register(self) -> CauldronRegistration<Self> {
            self.provide::<first::Strategy>()
                .export::<first::Strategy>()
                .global()
        }
    }
    #[cauldron]
    struct Bridge;
    impl Cauldron for Bridge {
        fn register(self) -> CauldronRegistration<Self> {
            self.import(GlobalStrategy)
        }
    }
    #[controller]
    struct GlobalController;
    impl Sealable for GlobalController {
        fn seals() -> SealRegistration<Self> {
            Self::seal::<SharedPolicy>()
        }
    }
    #[controller(route = "/global")]
    impl GlobalController {
        #[get]
        fn get(&self, principal: Authenticated<Principal>) -> &'static str {
            principal.label
        }
    }
    #[cauldron]
    struct GlobalRoot;
    impl Cauldron for GlobalRoot {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<GlobalController>().import(Bridge)
        }
    }
    async fn app<M: Cauldron>() -> Furnace {
        let mut builder = Furnace::builder_with_config(config());
        builder.root::<M>().unwrap();
        builder.build().await.unwrap()
    }
    async fn assert_request(app: &Furnace, path: &str, expected: u8, label: &str, method: Method) {
        let jwt = app
            .context()
            .resolve::<JwtService>()
            .unwrap()
            .sign(
                ScopedClaims { expected },
                JwtSignOptions::access(Duration::from_secs(60)),
            )
            .unwrap();
        let response = request(&build_router(app).unwrap(), method, path, Some(&jwt), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            label
        );
    }
    #[tokio::test]
    async fn a_shared_policy_preserves_each_controller_cauldron_and_application_binding() {
        let _lock = TEST_LOCK.lock().await;
        FIRST.store(0, Ordering::SeqCst);
        SECOND.store(0, Ordering::SeqCst);
        let both = app::<Both>().await;
        assert_request(&both, "/first", 1, "first", Method::GET).await;
        assert_request(&both, "/first", 1, "first", Method::POST).await;
        assert_request(&both, "/second", 2, "second", Method::GET).await;
        let first = app::<first::Module>().await;
        let second = app::<second::Module>().await;
        assert_request(&first, "/first", 1, "first", Method::GET).await;
        assert_request(&second, "/second", 2, "second", Method::GET).await;
        assert_eq!(FIRST.load(Ordering::SeqCst), 3);
        assert_eq!(SECOND.load(Ordering::SeqCst), 2);
    }
    #[tokio::test]
    async fn sealed_requests_can_use_direct_and_global_exported_strategies() {
        let _lock = TEST_LOCK.lock().await;
        assert_request(
            &app::<DirectRoot>().await,
            "/direct",
            1,
            "first",
            Method::GET,
        )
        .await;
        assert_request(
            &app::<GlobalRoot>().await,
            "/global",
            1,
            "first",
            Method::GET,
        )
        .await;
    }
}

#[tokio::test]
async fn skipped_endpoint_bypasses_authentication_and_policy_without_exposing_other_verbs() {
    let _lock = TEST_LOCK.lock().await;
    let app = application().await;
    let router = build_router(&app).unwrap();
    let denied = token(&app, false, false, false);
    let allowed = token(&app, true, true, true);
    for (method, credentials, status) in [
        (Method::GET, None, StatusCode::OK),
        (Method::GET, Some("malformed"), StatusCode::OK),
        (Method::GET, Some(denied.as_str()), StatusCode::OK),
        (Method::POST, None, StatusCode::UNAUTHORIZED),
        (Method::POST, Some("malformed"), StatusCode::UNAUTHORIZED),
        (Method::POST, Some(denied.as_str()), StatusCode::FORBIDDEN),
        (Method::POST, Some(allowed.as_str()), StatusCode::OK),
        (Method::DELETE, None, StatusCode::METHOD_NOT_ALLOWED),
        (Method::HEAD, None, StatusCode::OK),
    ] {
        let mut request = Request::builder().method(method).uri("/users/mixed");
        if let Some(value) = credentials {
            request = request.header(AUTHORIZATION, format!("Bearer {value}"));
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    for (credential, status) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(allowed.as_str()), StatusCode::OK),
    ] {
        let response = request(&router, Method::HEAD, "/users", credential, None).await;
        assert_eq!(response.status(), status);
    }
}

#[controller]
struct AllSkipped;
impl Sealable for AllSkipped {
    fn seals() -> SealRegistration<Self> {
        Self::seal::<Policy>()
    }
}
#[controller(route = "/all-public")]
impl AllSkipped {
    #[get]
    #[seal(skip)]
    fn index(&self) -> &'static str {
        "public"
    }
    #[cfg(any())]
    #[post]
    fn disabled_protected(&self) {}
}
#[cauldron]
struct AllSkippedRoot;
impl Cauldron for AllSkippedRoot {
    fn register(self) -> CauldronRegistration<Self> {
        self.controller::<AllSkipped>()
    }
}
#[tokio::test]
async fn entirely_skipped_controller_needs_no_jwt_output_or_configuration() {
    let mut builder = Furnace::builder();
    builder.root::<AllSkippedRoot>().unwrap();
    let app = builder.build().await.unwrap();
    assert!(app.context().resolve::<JwtService>().is_err());
    let response = build_router(&app)
        .unwrap()
        .oneshot(
            Request::builder()
                .uri("/all-public")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn focused_all_skipped_controller_needs_no_jwt_supply() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<AllSkipped>().unwrap();
    let app = builder.build().await.unwrap();
    let router = furnace_rs_common::__private::build_test_router_for::<AllSkipped>(&app).unwrap();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/all-public")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
