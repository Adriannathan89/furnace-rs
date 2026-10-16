//! Distributed live delivery through an isolated real Redis instance.
#![cfg(feature = "redis")]
#[path = "support/cluster.rs"]
mod cluster;
#[path = "support/redis.rs"]
mod redis_fixture;
mod support;
use cluster::*;
use serde_json::json;
#[tokio::test]
async fn distributes_live_room_messages_once_across_instances() {
    let redis = redis_fixture::Redis::start().await;
    let extra = [
        ("socket.redis.url", redis.url.as_str()),
        ("socket.namespace", "live-test"),
    ];
    let a = support::Server::configured::<App>(&extra).await;
    let b = support::Server::configured::<App>(&extra).await;
    let mut ca = a.connect("/chat").await;
    let mut cb = b.connect("/chat").await;
    for c in [&mut ca, &mut cb] {
        send(c, "furnace.join", json!({"room":"room"})).await;
        assert_eq!(recv(c).await["event"], "furnace.ok");
    }
    for n in 0..3 {
        send(&mut ca, "say", json!(n)).await;
        assert_eq!(recv(&mut ca).await["data"], n);
        assert_eq!(recv(&mut cb).await["data"], n);
    }
    ca.close(None).await.unwrap();
    cb.close(None).await.unwrap();
    a.stop().await;
    b.stop().await;
}

#[tokio::test]
async fn distributed_delete_requires_empty_membership_and_recreation_recovers() {
    let redis = redis_fixture::Redis::start().await;
    let extra = [
        ("socket.redis.url", redis.url.as_str()),
        ("socket.namespace", "delete-test"),
        ("socket.lease_refresh_ms", "30"),
        ("socket.lease_expiry_ms", "200"),
    ];
    let a = support::Server::configured::<App>(&extra).await;
    let b = support::Server::configured::<App>(&extra).await;
    let mut ca = a.connect("/chat").await;
    let mut cb = b.connect("/chat").await;
    send(&mut cb, "furnace.join", json!({"room":"room"})).await;
    assert_eq!(recv(&mut cb).await["event"], "furnace.ok");
    send(&mut ca, "delete", json!({})).await;
    assert_eq!(recv(&mut ca).await["data"]["code"], "room_not_empty");
    send(&mut cb, "furnace.leave", json!({"room":"room"})).await;
    assert_eq!(recv(&mut cb).await["event"], "furnace.ok");
    send(&mut ca, "delete", json!({})).await;
    assert_eq!(recv(&mut ca).await["event"], "deleted");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    send(&mut ca, "create", json!({})).await;
    assert_eq!(recv(&mut ca).await["event"], "created");
    send(&mut cb, "furnace.join", json!({"room":"room"})).await;
    assert_eq!(recv(&mut cb).await["event"], "furnace.ok");
    ca.close(None).await.unwrap();
    cb.close(None).await.unwrap();
    a.stop().await;
    b.stop().await;
}
#[tokio::test]
async fn redis_outage_keeps_direct_sends_and_rejects_room_sends() {
    let redis = redis_fixture::Redis::start().await;
    let url = redis.url.clone();
    let extra = [
        ("socket.redis.url", url.as_str()),
        ("socket.namespace", "outage-test"),
        ("socket.lease_refresh_ms", "30"),
        ("socket.lease_expiry_ms", "200"),
    ];
    let s = support::Server::configured::<App>(&extra).await;
    let mut c = s.connect("/chat").await;
    send(&mut c, "furnace.join", json!({"room":"room"})).await;
    assert_eq!(recv(&mut c).await["event"], "furnace.ok");
    drop(redis);
    send(&mut c, "say", json!(1)).await;
    assert_eq!(recv(&mut c).await["data"]["code"], "backend_unavailable");
    send(&mut c, "direct", json!({})).await;
    assert_eq!(recv(&mut c).await["event"], "direct");
    c.close(None).await.unwrap();
    s.stop().await;
}
