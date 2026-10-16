//! Replay and retention across independently reconnecting server instances.
#![cfg(feature = "redis")]
#[path = "support/cluster.rs"]
mod cluster;
#[path = "support/redis.rs"]
mod redis_fixture;
mod support;
use cluster::*;
use serde_json::json;
#[tokio::test]
async fn replays_retained_events_and_rejects_expired_cursors() {
    let redis = redis_fixture::Redis::start().await;
    let extra = [
        ("socket.redis.url", redis.url.as_str()),
        ("socket.namespace", "replay-test"),
        ("test.replay", "true"),
    ];
    let a = support::Server::configured::<App>(&extra).await;
    let b = support::Server::configured::<App>(&extra).await;
    let mut ca = a.connect("/chat").await;
    send(&mut ca, "furnace.join", json!({"room":"room"})).await;
    assert_eq!(recv(&mut ca).await["event"], "furnace.ok");
    send(&mut ca, "say", json!(0)).await;
    let first = recv(&mut ca).await;
    let cursor = first["cursor"].as_str().unwrap().to_owned();
    for n in 1..=2 {
        send(&mut ca, "say", json!(n)).await;
        assert_eq!(recv(&mut ca).await["data"], n);
    }
    let mut cb = b.connect("/chat").await;
    send(
        &mut cb,
        "furnace.join",
        json!({"room":"room","after":cursor}),
    )
    .await;
    assert_eq!(recv(&mut cb).await["data"], 1);
    assert_eq!(recv(&mut cb).await["data"], 2);
    assert_eq!(recv(&mut cb).await["event"], "furnace.ok");
    send(&mut ca, "say", json!(3)).await;
    assert_eq!(recv(&mut ca).await["data"], 3);
    assert_eq!(recv(&mut cb).await["data"], 3);
    let mut expired = b.connect("/chat").await;
    send(
        &mut expired,
        "furnace.join",
        json!({"room":"room","after":cursor}),
    )
    .await;
    assert_eq!(recv(&mut expired).await["data"]["code"], "history_expired");
    for c in [&mut ca, &mut cb, &mut expired] {
        c.close(None).await.unwrap();
    }
    a.stop().await;
    b.stop().await;
}

#[tokio::test]
async fn time_retention_expires_history_without_new_publications() {
    let redis = redis_fixture::Redis::start().await;
    let extra = [
        ("socket.redis.url", redis.url.as_str()),
        ("socket.namespace", "age-test"),
        ("test.replay", "true"),
        ("test.max_age_ms", "500"),
    ];
    let s = support::Server::configured::<App>(&extra).await;
    let mut ca = s.connect("/chat").await;
    send(&mut ca, "furnace.join", json!({"room":"room"})).await;
    assert_eq!(recv(&mut ca).await["event"], "furnace.ok");
    send(&mut ca, "say", json!(0)).await;
    let cursor = recv(&mut ca).await["cursor"].as_str().unwrap().to_owned();
    send(&mut ca, "say", json!(1)).await;
    assert_eq!(recv(&mut ca).await["data"], 1);
    tokio::time::sleep(std::time::Duration::from_millis(550)).await;
    let mut cb = s.connect("/chat").await;
    send(
        &mut cb,
        "furnace.join",
        json!({"room":"room","after":cursor}),
    )
    .await;
    assert_eq!(recv(&mut cb).await["data"]["code"], "history_expired");
    ca.close(None).await.unwrap();
    cb.close(None).await.unwrap();
    s.stop().await;
}
#[tokio::test]
async fn concurrent_publishers_have_identical_room_order_on_all_instances() {
    let redis = redis_fixture::Redis::start().await;
    let extra = [
        ("socket.redis.url", redis.url.as_str()),
        ("socket.namespace", "order-test"),
        ("test.replay", "true"),
        ("test.max_messages", "128"),
    ];
    let a = support::Server::configured::<App>(&extra).await;
    let b = support::Server::configured::<App>(&extra).await;
    let mut ca = a.connect("/chat").await;
    let mut cb = b.connect("/chat").await;
    for c in [&mut ca, &mut cb] {
        send(c, "furnace.join", json!({"room":"room"})).await;
        assert_eq!(recv(c).await["event"], "furnace.ok");
    }
    tokio::join!(
        async {
            for n in 0..20 {
                send(&mut ca, "say", json!(n)).await;
            }
        },
        async {
            for n in 20..40 {
                send(&mut cb, "say", json!(n)).await;
            }
        }
    );
    let mut messages = Vec::new();
    for c in [&mut ca, &mut cb] {
        let mut room = Vec::new();
        for _ in 0..40 {
            let e = recv(c).await;
            room.push((
                e["cursor"].as_str().unwrap().to_owned(),
                e["data"].as_u64().unwrap(),
            ));
        }
        assert_eq!(
            room.iter()
                .map(|(_, n)| *n)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            40
        );
        messages.push(room);
    }
    assert_eq!(messages[0], messages[1]);
    ca.close(None).await.unwrap();
    cb.close(None).await.unwrap();
    a.stop().await;
    b.stop().await;
}
