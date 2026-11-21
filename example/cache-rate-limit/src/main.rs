//! Redis-backed `#[cache]` and `#[rate_limit]` on controller endpoints.
//!
//! Set `REDIS_URL` to use Redis (shared by every instance); without it the
//! example falls back to the in-memory backend, which is per-process.

use furnace_rs::prelude::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

/// Counts how often the "expensive" handler really runs; cache hits do not.
static GENERATED: AtomicU64 = AtomicU64::new(0);

#[controller]
struct ArticleController;

#[controller(route = "/articles")]
impl ArticleController {
    /// At most 20 requests per minute per client; the response is cached for 30 s.
    ///
    /// Rate limiting sits in front of the cache, so cache hits still count.
    #[get]
    #[rate_limit(limit = 20, per = 60)]
    #[cache(ttl = 30)]
    async fn list(&self) -> Json<Value> {
        tokio::time::sleep(Duration::from_millis(300)).await; // stands in for a slow query
        let generated = GENERATED.fetch_add(1, Ordering::SeqCst) + 1;
        Json(json!({ "generated": generated, "articles": ["Hello", "World"] }))
    }
}

#[derive(Deserialize)]
struct Login {
    password: String,
}

#[controller]
struct AuthController;

#[controller(route = "/auth")]
impl AuthController {
    /// Brute-force protection: 5 attempts per minute per client IP.
    #[post("/login")]
    #[rate_limit(limit = 5, per = 60)]
    async fn login(&self, Json(body): Json<Login>) -> HttpResult<Json<Value>> {
        if body.password == "correct-horse" {
            Ok(Json(json!({ "ok": true })))
        } else {
            Err(Unauthorized::new("invalid credentials").into())
        }
    }
}

/// `ExtraCauldron` registers the `RedisExtra` slot that `provide` fills below.
#[cauldron(root, imports = [ExtraCauldron], controllers = [ArticleController, AuthController])]
struct AppCauldron;

/// Builds the application and its router with the given storage.
async fn build(
    extra: RedisExtra,
) -> Result<(Furnace, furnace_rs::axum::Router), Box<dyn std::error::Error>> {
    let mut builder = Furnace::builder();
    builder.root::<AppCauldron>()?;
    // A rooted builder only accepts `provide` for types a cauldron registers.
    builder.provide(extra)?;
    let application = builder.build().await?;
    let router = build_router(&application)?;
    Ok((application, router))
}

#[furnace_rs::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extra = match std::env::var("REDIS_URL") {
        Ok(url) => RedisExtra::new(&url)?,
        Err(_) => {
            eprintln!("REDIS_URL is not set: using the in-memory backend (this process only)");
            RedisExtra::with_backend(MemoryBackend::default())
        }
    };
    let (application, router) = build(extra).await?;
    serve_router(application, router, "127.0.0.1:3006").await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use furnace_rs::axum::{
        body::{Body, to_bytes},
        http::{Method, Request, StatusCode},
    };
    use tower::ServiceExt;

    async fn send(
        router: &furnace_rs::axum::Router,
        method: Method,
        uri: &str,
        body: &str,
    ) -> (StatusCode, String) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn articles_are_cached_and_login_is_rate_limited() {
        let (_application, router) = build(RedisExtra::with_backend(MemoryBackend::default()))
            .await
            .unwrap();

        // The second identical GET is a cache hit: the handler does not run again.
        let (_, first) = send(&router, Method::GET, "/articles", "").await;
        let (status, second) = send(&router, Method::GET, "/articles", "").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first, second);

        // Five wrong passwords are answered normally; the sixth attempt is limited.
        for _ in 0..5 {
            let (status, _) = send(
                &router,
                Method::POST,
                "/auth/login",
                r#"{"password":"nope"}"#,
            )
            .await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
        }
        let (status, _) = send(
            &router,
            Method::POST,
            "/auth/login",
            r#"{"password":"correct-horse"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    }
}
