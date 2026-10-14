//! Checked response-cookie composition.

#![cfg(feature = "cookies")]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::SET_COOKIE},
    routing::get,
};
use furnace_rs_common::{
    Cookie, CookieError, CookieErrorKind, CookieJar, CookieRejection, SameSite, cookie::time,
};
use tower::ServiceExt;

async fn assert_internal_cookie_rejection(response: axum::response::Response, sentinels: &[&str]) {
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response.headers().get_all(SET_COOKIE).iter().count(), 0);
    for sentinel in sentinels {
        assert!(
            response
                .headers()
                .values()
                .all(|value| { !String::from_utf8_lossy(value.as_bytes()).contains(sentinel) })
        );
    }

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = std::str::from_utf8(&body).unwrap();
    assert_eq!(
        body,
        "{\"error\":{\"code\":\"internal\",\"message\":\"internal server error\"}}"
    );
    for sentinel in sentinels {
        assert!(!body.contains(sentinel));
    }
}

#[tokio::test]
async fn tuple_response_emits_each_cookie_with_all_attributes() {
    async fn handler(jar: CookieJar) -> (CookieJar, &'static str) {
        let access = Cookie::build(("access", "token-a"))
            .path("/")
            .domain("example.com")
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .max_age(time::Duration::minutes(15))
            .build();
        let refresh = Cookie::build(("refresh", "token-b"))
            .path("/auth/refresh")
            .expires(time::OffsetDateTime::now_utc() + time::Duration::days(7))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .build();
        (jar.add(access).add(refresh), "ok")
    }

    let response = Router::new()
        .route("/", get(handler))
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let values = response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(values.len(), 2);
    assert!(values.iter().any(|value| {
        value.contains("access=")
            && value.contains("Path=/")
            && value.contains("Domain=example.com")
            && value.contains("HttpOnly")
            && value.contains("Secure")
            && value.contains("SameSite=Strict")
            && value.contains("Max-Age=900")
    }));
    assert!(values.iter().any(|value| {
        value.contains("refresh=")
            && value.contains("Path=/auth/refresh")
            && value.contains("Expires=")
            && value.contains("HttpOnly")
            && value.contains("Secure")
            && value.contains("SameSite=Lax")
    }));
}

#[tokio::test]
async fn removal_emits_an_expired_deletion_cookie() {
    async fn handler(jar: CookieJar) -> (CookieJar, &'static str) {
        (jar.remove(Cookie::new("session", "ignored")), "ok")
    }

    let response = Router::new()
        .route("/", get(handler))
        .oneshot(
            Request::builder()
                .uri("/")
                .header("cookie", "session=old")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let value = response
        .headers()
        .get(SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();

    assert!(value.starts_with("session="));
    assert!(value.contains("Max-Age=0"));
    assert!(value.contains("Expires="));
}

#[tokio::test]
async fn malformed_set_cookie_header_values_are_rejected_without_disclosure() {
    async fn invalid_path(jar: CookieJar) -> (CookieJar, &'static str) {
        (
            jar.add(Cookie::build(("private", "secret")).path("/\n").build()),
            "ok",
        )
    }
    async fn invalid_domain(jar: CookieJar) -> (CookieJar, &'static str) {
        (
            jar.add(
                Cookie::build(("private", "secret"))
                    .domain("example.com\r\n")
                    .build(),
            ),
            "ok",
        )
    }

    for app in [
        Router::new().route("/", get(invalid_path)),
        Router::new().route("/", get(invalid_domain)),
    ] {
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_internal_cookie_rejection(response, &["private", "secret"]).await;
    }
}

#[tokio::test]
async fn same_site_none_requires_secure_unless_unspecified() {
    async fn invalid(jar: CookieJar) -> (CookieJar, &'static str) {
        (
            jar.add(
                Cookie::build(("session", "token"))
                    .same_site(SameSite::None)
                    .secure(false)
                    .build(),
            ),
            "ok",
        )
    }
    async fn valid(jar: CookieJar) -> (CookieJar, &'static str) {
        (
            jar.add(
                Cookie::build(("session", "token"))
                    .same_site(SameSite::None)
                    .secure(true)
                    .build(),
            ),
            "ok",
        )
    }
    async fn unspecified(jar: CookieJar) -> (CookieJar, &'static str) {
        (
            jar.add(
                Cookie::build(("session", "token"))
                    .same_site(SameSite::None)
                    .build(),
            ),
            "ok",
        )
    }

    let invalid_response = Router::new()
        .route("/", get(invalid))
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_internal_cookie_rejection(invalid_response, &["session", "token"]).await;

    let valid_response = Router::new()
        .route("/", get(valid))
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let value = valid_response
        .headers()
        .get(SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(value.contains("SameSite=None"));
    assert!(value.contains("Secure"));

    let unspecified_response = Router::new()
        .route("/", get(unspecified))
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(unspecified_response.status(), StatusCode::OK);
    let value = unspecified_response
        .headers()
        .get(SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(value.contains("SameSite=None"));
    assert!(value.contains("Secure"));
}

#[tokio::test]
async fn cookie_names_and_values_are_percent_encoded_into_valid_headers() {
    async fn handler(jar: CookieJar) -> (CookieJar, &'static str) {
        (jar.add(Cookie::new("name with space", "line\nbreak")), "ok")
    }

    let response = Router::new()
        .route("/", get(handler))
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let value = response
        .headers()
        .get(SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(value, "name%20with%20space=line%0Abreak");
}

#[tokio::test]
async fn invalid_batch_is_atomic() {
    async fn handler(jar: CookieJar) -> (CookieJar, &'static str) {
        let valid = Cookie::new("access", "valid-token");
        let invalid = Cookie::build(("refresh", "private-token"))
            .path("/\n")
            .build();
        (jar.add(valid).add(invalid), "ok")
    }

    let response = Router::new()
        .route("/", get(handler))
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_internal_cookie_rejection(
        response,
        &["access", "valid-token", "refresh", "private-token"],
    )
    .await;
}

async fn assert_attribute_injection_rejected(domain: bool, removal: bool) {
    use axum::response::IntoResponse;

    let injected = Cookie::build(("private-cookie", "private-token"))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict);
    let injected = if domain {
        injected.domain("example.com; SameSite=None; Max-Age=31536000")
    } else {
        injected.path("/; SameSite=None; Max-Age=31536000")
    }
    .build();
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::COOKIE,
        "private-cookie=old-token".parse().unwrap(),
    );
    let jar = CookieJar::from_headers(&headers)
        .unwrap()
        .add(Cookie::new("valid-cookie", "valid-token"));
    let jar = if removal {
        jar.remove(injected)
    } else {
        jar.add(injected)
    };
    let response = jar.into_response();
    assert_internal_cookie_rejection(
        response,
        &[
            "private-cookie",
            "private-token",
            "valid-cookie",
            "valid-token",
        ],
    )
    .await;
}

#[tokio::test]
async fn domain_attribute_injection_is_rejected_for_add() {
    assert_attribute_injection_rejected(true, false).await;
}

#[tokio::test]
async fn domain_attribute_injection_is_rejected_for_removal() {
    assert_attribute_injection_rejected(true, true).await;
}

#[tokio::test]
async fn path_attribute_injection_is_rejected_for_add() {
    assert_attribute_injection_rejected(false, false).await;
}

#[tokio::test]
async fn path_attribute_injection_is_rejected_for_removal() {
    assert_attribute_injection_rejected(false, true).await;
}

#[tokio::test]
async fn ascii_controls_in_attributes_are_rejected_for_add_and_removal() {
    use axum::response::IntoResponse;

    for control in ['\t', '\0', '\u{1f}', '\u{7f}'] {
        for domain in [false, true] {
            for removal in [false, true] {
                let cookie = Cookie::build(("private-cookie", "private-token"));
                let cookie = if domain {
                    cookie.domain(format!("example.com{control}"))
                } else {
                    cookie.path(format!("/{control}"))
                }
                .build();
                let mut headers = axum::http::HeaderMap::new();
                headers.insert(
                    axum::http::header::COOKIE,
                    "private-cookie=old-token".parse().unwrap(),
                );
                let jar = CookieJar::from_headers(&headers)
                    .unwrap()
                    .add(Cookie::new("valid-cookie", "valid-token"));
                let jar = if removal {
                    jar.remove(cookie)
                } else {
                    jar.add(cookie)
                };
                assert_internal_cookie_rejection(
                    jar.into_response(),
                    &[
                        "private-cookie",
                        "private-token",
                        "valid-cookie",
                        "valid-token",
                    ],
                )
                .await;
            }
        }
    }
}

#[tokio::test]
async fn response_errors_are_redacted_and_stably_classified() {
    use axum::response::IntoResponse;

    const NAME_SENTINEL: &str = "sentinel_name";
    const VALUE_SENTINEL: &str = "sentinel_value";
    let jar = CookieJar::new().add(
        Cookie::build((NAME_SENTINEL, VALUE_SENTINEL))
            .path("/\n")
            .build(),
    );
    let debug = format!("{jar:?}");
    assert!(!debug.contains(NAME_SENTINEL));
    assert!(!debug.contains(VALUE_SENTINEL));
    assert_internal_cookie_rejection(jar.into_response(), &[NAME_SENTINEL, VALUE_SENTINEL]).await;

    let rejection = CookieRejection::from(CookieError::new(CookieErrorKind::InvalidResponse));
    let display = rejection.to_string();
    let debug = format!("{rejection:?}");
    assert_eq!(display, "response cookie is invalid");
    assert!(!display.contains(NAME_SENTINEL));
    assert!(!display.contains(VALUE_SENTINEL));
    assert!(!debug.contains(NAME_SENTINEL));
    assert!(!debug.contains(VALUE_SENTINEL));
}
