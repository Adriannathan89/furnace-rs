//! Handshake credentials, identity, and browser Origin policy.
#![cfg(feature = "jwt")]
mod support;
use furnace_rs_core::{ConfigBuilder, Injector, MapSource, cauldron};
use furnace_rs_socket::common::{
    ClaimsPrincipal, JwtService, JwtSignOptions, PassportPrincipal, guard,
};
use furnace_rs_socket::{SocketContext, SocketResult, socket_gateway};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue};
#[derive(Serialize, Deserialize)]
struct Claims {
    id: u64,
}
impl PassportPrincipal for Claims {
    fn has_role(&self, _: &str) -> bool {
        false
    }
    fn has_permission(&self, _: &str) -> bool {
        false
    }
}
#[guard(strategy="jwt",principal=ClaimsPrincipal<Claims>)]
struct Protected;
struct Gateway;
impl Injector for Gateway {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
#[socket_gateway(path="/private",mode="events",guard=Protected)]
impl Gateway {
    #[on_connect]
    async fn connected(&self, ctx: SocketContext) -> SocketResult<()> {
        let p = ctx.handshake().authenticated::<ClaimsPrincipal<Claims>>()?;
        ctx.emit("identity", serde_json::json!(p.id)).await
    }
}
#[cauldron(root,socket_gateways=[Gateway])]
struct App;
const SECRET: &str = "test-secret-with-at-least-thirty-two-characters";
fn token() -> String {
    let c = ConfigBuilder::new()
        .source(MapSource::new("test", [("passport.secret", SECRET)]))
        .build()
        .unwrap();
    JwtService::from_config(&c)
        .unwrap()
        .sign(
            Claims { id: 42 },
            JwtSignOptions::access(std::time::Duration::from_secs(60)),
        )
        .unwrap()
}
#[tokio::test]
async fn authenticates_each_source_and_rejects_ambiguity() {
    let server = support::Server::configured::<App>(&[("passport.secret", SECRET)]).await;
    let token = token();
    for source in ["header", "cookie", "query"] {
        let path = if source == "query" {
            format!("/private?access_token={token}")
        } else {
            "/private".into()
        };
        let mut r = format!("ws://{}{path}", server.address)
            .into_client_request()
            .unwrap();
        if source == "header" {
            r.headers_mut().insert(
                "authorization",
                HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
            );
        }
        if source == "cookie" {
            r.headers_mut().insert(
                "cookie",
                HeaderValue::from_str(&format!("other=ok; access_token={token}")).unwrap(),
            );
        }
        let (mut client, _) = tokio_tungstenite::connect_async(r).await.unwrap();
        let v = client.next().await.unwrap().unwrap().into_text().unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&v).unwrap()["data"],
            42
        );
        client.close(None).await.unwrap();
    }
    for path in [
        "/private".to_owned(),
        "/private?access_token=invalid".into(),
        format!("/private?access_token={token}&access_token={token}"),
    ] {
        assert!(
            tokio_tungstenite::connect_async(format!("ws://{}{path}", server.address))
                .await
                .is_err()
        );
    }
    let mut r = format!("ws://{}/private?access_token={token}", server.address)
        .into_client_request()
        .unwrap();
    r.headers_mut().insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    assert!(tokio_tungstenite::connect_async(r).await.is_err());
    let mut r = format!("ws://{}/private?access_token={token}", server.address)
        .into_client_request()
        .unwrap();
    r.headers_mut()
        .insert("origin", HeaderValue::from_static("https://untrusted.test"));
    assert!(tokio_tungstenite::connect_async(r).await.is_err());
    server.stop().await;
}

#[tokio::test]
async fn origin_allowlist_is_exact_and_duplicate_origins_are_rejected() {
    let server = support::Server::with_origins::<App>(
        &[("passport.secret", SECRET)],
        &["https://app.example:8443"],
    )
    .await;
    let token = token();
    for origin in [
        None,
        Some("https://app.example:8443"),
        Some("https://app.example"),
        Some("http://app.example:8443"),
        Some("https://app.example:8443/"),
        Some("null"),
    ] {
        let mut r = format!("ws://{}/private?access_token={token}", server.address)
            .into_client_request()
            .unwrap();
        if let Some(origin) = origin {
            r.headers_mut()
                .insert("origin", HeaderValue::from_str(origin).unwrap());
        }
        let result = tokio_tungstenite::connect_async(r).await;
        let allowed = origin.is_none() || origin == Some("https://app.example:8443");
        assert_eq!(result.is_ok(), allowed, "{origin:?}");
        if let Ok((mut c, _)) = result {
            c.close(None).await.unwrap();
        }
    }
    let mut r = format!("ws://{}/private?access_token={token}", server.address)
        .into_client_request()
        .unwrap();
    for _ in 0..2 {
        r.headers_mut().append(
            "origin",
            HeaderValue::from_static("https://app.example:8443"),
        );
    }
    assert!(tokio_tungstenite::connect_async(r).await.is_err());
    server.stop().await;
}
#[tokio::test]
async fn conflicting_repeated_and_encoded_credentials_never_fall_back() {
    let server = support::Server::configured::<App>(&[("passport.secret", SECRET)]).await;
    let token = token();
    for (authorization, cookie, query) in [
        (
            Some(format!("Bearer {token}")),
            Some(format!("access_token={token}")),
            String::new(),
        ),
        (
            None,
            Some(format!("access_token={token}")),
            format!("?access_token={token}"),
        ),
        (
            Some("Basic invalid".into()),
            Some(format!("access_token={token}")),
            String::new(),
        ),
        (
            None,
            Some(format!("access_token={token}; access_token={token}")),
            String::new(),
        ),
        (
            None,
            None,
            format!("?%61ccess_token={token}&access_token={token}"),
        ),
    ] {
        let mut r = format!("ws://{}/private{query}", server.address)
            .into_client_request()
            .unwrap();
        if let Some(v) = authorization {
            r.headers_mut()
                .insert("authorization", HeaderValue::from_str(&v).unwrap());
        }
        if let Some(v) = cookie {
            r.headers_mut()
                .insert("cookie", HeaderValue::from_str(&v).unwrap());
        }
        let failure = tokio_tungstenite::connect_async(r).await.unwrap_err();
        assert!(!format!("{failure:?}").contains(&token));
    }
    let mut r = format!("ws://{}/private", server.address)
        .into_client_request()
        .unwrap();
    for _ in 0..2 {
        r.headers_mut().append(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
    }
    assert!(tokio_tungstenite::connect_async(r).await.is_err());
    server.stop().await;
}
