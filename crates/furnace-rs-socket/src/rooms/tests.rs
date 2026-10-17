//! Deterministic room transition regressions using a real Redis server.
use super::redis::RedisBackend;
use crate::{hub::sender, *};
use furnace_rs_common::runtime::RuntimeTasks;
use furnace_rs_core::{ConfigBuilder, Furnace, MapSource, cauldron};
use std::sync::Arc;
use tokio::sync::Notify;
#[path = "../../tests/support/redis.rs"]
mod redis_fixture;
pub(super) struct Pause {
    pub name: &'static str,
    pub entered: Arc<Notify>,
    pub release: Arc<Notify>,
}
fn pause(backend: &RedisBackend, name: &'static str) -> (Arc<Notify>, Arc<Notify>) {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    *backend.pause.lock().unwrap() = Some(Pause {
        name,
        entered: entered.clone(),
        release: release.clone(),
    });
    (entered, release)
}
#[cauldron(root)]
struct App;
struct Allow;
impl crate::dispatch::SocketGateway for Allow {
    fn call(
        &self,
        _: SocketContext,
        _: crate::dispatch::SocketInput,
    ) -> futures_util::future::BoxFuture<'_, SocketResult<()>> {
        Box::pin(async { Ok(()) })
    }
}
struct Fixture {
    _redis: redis_fixture::Redis,
    _shutdown: tokio::sync::watch::Sender<bool>,
    backend: Arc<RedisBackend>,
    context: SocketContext,
    output: tokio::sync::mpsc::Receiver<crate::hub::Queued>,
}
impl Fixture {
    async fn new(replay: bool) -> Self {
        let redis = redis_fixture::Redis::start().await;
        let config = ConfigBuilder::new()
            .source(MapSource::new(
                "test",
                [
                    ("socket.address", "127.0.0.1:0"),
                    ("socket.redis.url", redis.url.as_str()),
                    ("socket.namespace", "transitions"),
                ],
            ))
            .build()
            .unwrap();
        let mut builder = Furnace::builder_with_config(config.clone());
        builder.root::<App>().unwrap();
        let app = builder.build().await.unwrap();
        let config = SocketServerConfig::parse(&config).unwrap();
        let hub = SocketHub::default();
        let backend = RedisBackend::prepare(&config, &hub).await.unwrap();
        hub.state.lock().unwrap().backend = Some(backend.clone());
        let id = ConnectionId("test-connection".into());
        let (tx, output, _) = sender(&config, EndpointMode::Events);
        hub.state
            .lock()
            .unwrap()
            .connections
            .insert(("/test".into(), id.clone()), tx);
        let (shutdown, stopped) = tokio::sync::watch::channel(false);
        let context = SocketContext {
            transitions: Arc::new(tokio::sync::Mutex::new(())),
            id,
            handshake: SocketHandshake {
                application: app.context().clone(),
                headers: Default::default(),
                method: http::Method::GET,
                uri: "/test".parse().unwrap(),
                peer: None,
                extensions: Arc::new(Default::default()),
            },
            hub: hub.gateway("/test"),
            gateway: Arc::new(Allow),
            acknowledge_join: true,
            request_id: None,
            tasks: RuntimeTasks::new(),
            shutdown: stopped,
        };
        context
            .hub()
            .create_room(
                "room",
                if replay {
                    RoomMode::Replayable {
                        max_age: std::time::Duration::from_secs(60),
                        max_messages: 100,
                    }
                } else {
                    RoomMode::Live
                },
            )
            .await
            .unwrap();
        Self {
            _redis: redis,
            _shutdown: shutdown,
            backend,
            context,
            output,
        }
    }
    async fn next(&mut self) -> serde_json::Value {
        let queued = tokio::time::timeout(std::time::Duration::from_secs(2), self.output.recv())
            .await
            .unwrap()
            .unwrap();
        match queued.message {
            Message::Text(s) => serde_json::from_str(&s).unwrap(),
            other => panic!("{other:?}"),
        }
    }
    async fn reset_generation(&self) {
        let mut connection = self
            .backend
            .client
            .get_multiplexed_async_connection()
            .await
            .unwrap();
        let keys = self.backend.keys("/test", "room");
        // Model expired memberships followed by an empty-only deletion/recreation.
        let _: i64 = redis::cmd("DEL")
            .arg(&keys[..3])
            .query_async(&mut connection)
            .await
            .unwrap();
        self.backend
            .create("/test", "room", &RoomMode::Live)
            .await
            .unwrap();
    }
}
#[tokio::test]
async fn leave_waits_for_inflight_replay_join_and_stops_its_reader() {
    let mut f = Fixture::new(true).await;
    let (entered, release) = pause(&f.backend, "replay_join");
    let c = f.context.clone();
    let join = tokio::spawn(async move { c.join("room", None).await });
    entered.notified().await;
    let c = f.context.clone();
    let mut leave = tokio::spawn(async move { c.leave("room").await });
    let completed_early = tokio::time::timeout(std::time::Duration::from_millis(50), &mut leave)
        .await
        .is_ok();
    release.notify_one();
    join.await.unwrap().unwrap();
    if !completed_early {
        leave.await.unwrap().unwrap();
    }
    assert!(
        !completed_early,
        "leave completed before replay join could install its reader"
    );
    assert_eq!(f.next().await["event"], "furnace.ok");
    f.context
        .hub()
        .emit_room("room", "message", serde_json::json!(1))
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), f.output.recv())
            .await
            .is_err()
    );
    f.context.tasks.abort_and_wait().await;
}
#[tokio::test]
async fn replay_rejoin_stops_old_reader_before_catchup() {
    let mut f = Fixture::new(true).await;
    f.context.join("room", None).await.unwrap();
    assert_eq!(f.next().await["event"], "furnace.ok");
    let cursor = f
        .context
        .hub()
        .emit_room("room", "message", serde_json::json!(0))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(f.next().await["data"], 0);
    f.context
        .hub()
        .emit_room("room", "message", serde_json::json!(1))
        .await
        .unwrap();
    assert_eq!(f.next().await["data"], 1);
    let (entered, release) = pause(&f.backend, "replay_join");
    let c = f.context.clone();
    let join = tokio::spawn(async move { c.join("room", Some(&cursor)).await });
    entered.notified().await;
    f.context
        .hub()
        .emit_room("room", "message", serde_json::json!(2))
        .await
        .unwrap();
    let leaked = tokio::time::timeout(std::time::Duration::from_millis(350), f.output.recv())
        .await
        .is_ok();
    release.notify_one();
    join.await.unwrap().unwrap();
    assert!(!leaked, "old reader delivered during replacement catchup");
    assert_eq!(f.next().await["data"], 1);
    assert_eq!(f.next().await["event"], "furnace.ok");
    assert_eq!(f.next().await["data"], 2);
    f.context.tasks.abort_and_wait().await;
}
#[tokio::test]
async fn restoration_retires_generation_lost_between_info_and_renewal() {
    let f = Fixture::new(false).await;
    f.context.join("room", None).await.unwrap();
    let (entered, release) = pause(&f.backend, "restore_info");
    let backend = f.backend.clone();
    let restore = tokio::spawn(async move { backend.restore().await });
    entered.notified().await;
    f.reset_generation().await;
    release.notify_one();
    restore.await.unwrap().unwrap();
    assert!(
        !f.context
            .hub
            .hub
            .state
            .lock()
            .unwrap()
            .rooms
            .contains_key(&("/test".into(), "room".into())),
        "stale local generation survived failed renewal"
    );
}
#[tokio::test]
async fn new_generation_join_does_not_inherit_stale_members() {
    let f = Fixture::new(false).await;
    f.context.join("room", None).await.unwrap();
    f.reset_generation().await;
    let mut fresh = f.context.clone();
    fresh.id = ConnectionId("fresh".into());
    fresh.acknowledge_join = false;
    fresh.join("room", None).await.unwrap();
    let state = f.context.hub.hub.state.lock().unwrap();
    assert!(
        !state.rooms[&("/test".into(), "room".into())]
            .members
            .contains(f.context.id()),
        "new generation inherited stale member"
    );
}

#[tokio::test]
async fn raw_generation_retirement_never_injects_json() {
    let mut f = Fixture::new(false).await;
    f.context.join("room", None).await.unwrap();
    assert_eq!(f.next().await["event"], "furnace.ok");
    let closed = {
        let mut state = f.context.hub.hub.state.lock().unwrap();
        let sender = state
            .connections
            .get_mut(&("/test".into(), f.context.id().clone()))
            .unwrap();
        sender.mode = EndpointMode::Raw;
        sender.close.subscribe()
    };
    f.reset_generation().await;
    f.backend.restore().await.unwrap();
    assert!(
        f.output.try_recv().is_err(),
        "retirement injected a text envelope into raw protocol"
    );
    assert!(*closed.borrow());
}
