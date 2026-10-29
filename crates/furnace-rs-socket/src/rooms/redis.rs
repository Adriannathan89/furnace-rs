//! Redis-backed room metadata, membership leases, and live fan-out.
use crate::{
    ConnectionId, Message, RoomMode, SocketError, SocketHub, SocketResult, SocketServerConfig,
};
use furnace_rs_common::runtime::RuntimeTasks;
use futures_util::StreamExt;
use std::{
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;

static INSTANCE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct RedisBackend {
    #[cfg(test)]
    pub(super) pause: Mutex<Option<super::tests::Pause>>,
    pub(crate) membership: tokio::sync::Mutex<()>,
    pub client: redis::Client,
    connection: Mutex<redis::aio::MultiplexedConnection>,
    subscription: Mutex<Option<redis::aio::PubSub>>,
    state: Weak<Mutex<crate::hub::HubState>>,
    pub prefix: String,
    pub instance: String,
    ready: AtomicBool,
    deadline: Mutex<Instant>,
    config: SocketServerConfig,
}

fn encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn unavailable<T>(_: T) -> SocketError {
    SocketError::new("backend_unavailable")
}

impl RedisBackend {
    pub async fn prepare(config: &SocketServerConfig, hub: &SocketHub) -> SocketResult<Arc<Self>> {
        if config
            .redis_url
            .as_deref()
            .is_some_and(|url| url.starts_with("rediss://"))
        {
            ensure_crypto_provider();
        }

        let client = redis::Client::open(
            config
                .redis_url
                .as_deref()
                .ok_or_else(|| SocketError::new("backend_unavailable"))?,
        )
        .map_err(unavailable)?;
        let connection = tokio::time::timeout(
            config.lease_refresh,
            client.get_multiplexed_async_connection(),
        )
        .await
        .map_err(unavailable)?
        .map_err(unavailable)?;
        let prefix = format!("furnace:socket:{}", encode(&config.namespace));
        let instance = format!(
            "{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            INSTANCE.fetch_add(1, Ordering::Relaxed)
        );
        let backend = Arc::new(Self {
            #[cfg(test)]
            pause: Mutex::new(None),
            membership: tokio::sync::Mutex::new(()),
            client,
            connection: Mutex::new(connection),
            subscription: Mutex::new(None),
            state: Arc::downgrade(&hub.state),
            prefix,
            instance,
            ready: AtomicBool::new(false),
            deadline: Mutex::new(Instant::now()),
            config: config.clone(),
        });
        let subscription = backend.subscribe().await?;
        backend.renew().await?;
        *backend.subscription.lock().unwrap() = Some(subscription);
        backend.ready.store(true, Ordering::Release);

        Ok(backend)
    }

    #[cfg(test)]
    pub(super) async fn checkpoint(&self, name: &str) {
        let pause = {
            let mut slot = self.pause.lock().unwrap();

            if slot.as_ref().is_some_and(|p| p.name == name) {
                slot.take()
            } else {
                None
            }
        };

        if let Some(pause) = pause {
            pause.entered.notify_one();
            pause.release.notified().await;
        }
    }

    pub fn healthy(&self) -> bool {
        self.ready.load(Ordering::Acquire) && Instant::now() < *self.deadline.lock().unwrap()
    }

    fn channel(&self) -> String {
        format!("{}:live", self.prefix)
    }

    fn lease(&self) -> String {
        format!("{}:lease:{}", self.prefix, self.instance)
    }

    pub fn keys(&self, path: &str, room: &str) -> Vec<String> {
        let key = format!("{}:{}:{}", self.prefix, encode(path), encode(room));

        vec![
            format!("{key}:meta"),
            format!("{key}:members"),
            format!("{key}:stream"),
            self.lease(),
            self.channel(),
        ]
    }

    async fn subscribe(&self) -> SocketResult<redis::aio::PubSub> {
        let mut subscription =
            tokio::time::timeout(self.config.lease_refresh, self.client.get_async_pubsub())
                .await
                .map_err(unavailable)?
                .map_err(unavailable)?;
        subscription
            .subscribe(self.channel())
            .await
            .map_err(unavailable)?;

        Ok(subscription)
    }

    async fn renew(&self) -> SocketResult<()> {
        let sent = Instant::now();
        let mut connection = self.connection.lock().unwrap().clone();
        let (): () = tokio::time::timeout(
            self.config.lease_refresh,
            redis::cmd("SET")
                .arg(self.lease())
                .arg(&self.instance)
                .arg("PX")
                .arg(self.config.lease_expiry.as_millis() as u64)
                .query_async(&mut connection),
        )
        .await
        .map_err(unavailable)?
        .map_err(unavailable)?;
        *self.deadline.lock().unwrap() = sent + self.config.lease_expiry;

        Ok(())
    }

    pub async fn command(
        &self,
        path: &str,
        room: &str,
        op: &str,
        args: &[String],
    ) -> SocketResult<redis::Value> {
        if !self.healthy() {
            return Err(SocketError::new("backend_unavailable"));
        }

        self.execute(path, room, op, args).await
    }

    async fn execute(
        &self,
        path: &str,
        room: &str,
        op: &str,
        args: &[String],
    ) -> SocketResult<redis::Value> {
        let mut command = redis::cmd("EVAL");
        command
            .arg(include_str!("scripts/rooms.lua"))
            .arg(5)
            .arg(self.keys(path, room))
            .arg(op)
            .arg(&self.instance)
            .arg(args);
        let mut connection = self.connection.lock().unwrap().clone();
        let value: redis::Value = tokio::time::timeout(
            self.config.lease_refresh,
            command.query_async(&mut connection),
        )
        .await
        .map_err(|_| {
            self.ready.store(false, Ordering::Release);

            SocketError::new("backend_unavailable")
        })?
        .map_err(|_| {
            self.ready.store(false, Ordering::Release);

            SocketError::new("backend_unavailable")
        })?;

        if let redis::Value::Array(values) = &value
            && values
                .first()
                .and_then(|v| redis::from_redis_value::<String>(v).ok())
                .as_deref()
                == Some("error")
        {
            let code = values
                .get(1)
                .and_then(|v| redis::from_redis_value::<String>(v).ok())
                .unwrap_or_default();

            return Err(SocketError::new(match code.as_str() {
                "room_not_found" => "room_not_found",
                "room_conflict" => "room_conflict",
                "room_not_empty" => "room_not_empty",
                "history_expired" => "history_expired",
                "invalid_cursor" => "invalid_cursor",
                "invalid_message" => "invalid_message",
                _ => "backend_unavailable",
            }));
        }

        Ok(value)
    }

    pub async fn create(&self, path: &str, room: &str, mode: &RoomMode) -> SocketResult<()> {
        let (age, count, replay) = match mode {
            RoomMode::Live => (0, 0, 0),
            RoomMode::Replayable {
                max_age,
                max_messages,
            } => (
                u64::try_from(max_age.as_millis())
                    .map_err(|_| SocketError::new("invalid_retention"))?,
                *max_messages,
                1,
            ),
        };
        self.command(
            path,
            room,
            "create",
            &[
                serde_json::to_string(mode)?,
                format!(
                    "{}-{}",
                    self.instance,
                    INSTANCE.fetch_add(1, Ordering::Relaxed)
                ),
                age.to_string(),
                count.to_string(),
                replay.to_string(),
            ],
        )
        .await?;

        Ok(())
    }

    pub async fn info(&self, path: &str, room: &str) -> SocketResult<RoomInfo> {
        let value = self.command(path, room, "info", &[]).await?;
        let fields: Vec<String> = redis::from_redis_value(&value).map_err(unavailable)?;

        Ok(RoomInfo {
            mode: serde_json::from_str(&fields[0])?,
            generation: fields[1].clone(),
            high: fields[2].clone(),
        })
    }

    pub async fn delete(&self, path: &str, room: &str) -> SocketResult<()> {
        self.command(path, room, "delete", &[]).await?;

        Ok(())
    }

    pub async fn join(
        &self,
        path: &str,
        room: &str,
        id: &ConnectionId,
        after: Option<&str>,
    ) -> SocketResult<RoomInfo> {
        if let Some(cursor) = after {
            super::replay::parse_id(cursor)?;
        }

        let value = self
            .command(
                path,
                room,
                "join",
                &[id.to_string(), after.unwrap_or("").to_owned()],
            )
            .await?;
        let fields: Vec<String> = redis::from_redis_value(&value).map_err(unavailable)?;

        Ok(RoomInfo {
            mode: serde_json::from_str(&fields[0])?,
            generation: fields[1].clone(),
            high: fields[2].clone(),
        })
    }

    pub async fn leave(&self, path: &str, room: &str, id: &ConnectionId) -> SocketResult<()> {
        self.command(path, room, "leave", &[id.to_string()]).await?;

        Ok(())
    }

    pub async fn publish(
        &self,
        path: &str,
        room: Option<&str>,
        message: Message,
    ) -> SocketResult<()> {
        let packet = Packet::new(path, room, message)?;
        self.command(
            path,
            room.unwrap_or(""),
            if room.is_some() {
                "publish"
            } else {
                "broadcast"
            },
            &[serde_json::to_string(&packet)?],
        )
        .await?;

        Ok(())
    }

    pub async fn append(
        &self,
        path: &str,
        room: &str,
        envelope: crate::SocketEnvelope,
    ) -> SocketResult<String> {
        let value = self
            .command(path, room, "append", &[envelope.to_text()?])
            .await?;

        redis::from_redis_value(&value).map_err(unavailable)
    }

    pub fn start(self: Arc<Self>, mut shutdown: watch::Receiver<bool>, tasks: &RuntimeTasks) {
        tasks.spawn(async move{
            let mut subscription=self.subscription.lock().unwrap().take();
            let mut interval=tokio::time::interval(self.config.lease_refresh);

            loop {
                if *shutdown.borrow(){break;}

                if subscription.is_none()||!self.healthy(){
                    self.ready.store(false,Ordering::Release);
                    let reconnect=async {
                        let connection=self.client.get_multiplexed_async_connection().await.map_err(unavailable)?;*self.connection.lock().unwrap()=connection;
                        let sub=self.subscribe().await?;self.renew().await?;
                        self.restore().await?;self.ready.store(true,Ordering::Release);Ok::<_,SocketError>(sub)
                    };
                    tokio::select!{biased;_=shutdown.changed()=>break,result=tokio::time::timeout(self.config.lease_refresh,reconnect)=>match result{Ok(Ok(sub))=>subscription=Some(sub),_=>{self.ready.store(false,Ordering::Release);tokio::select!{_=shutdown.changed()=>break,_=tokio::time::sleep(Duration::from_millis(200))=>{}}continue;}}}
                }

                let sub=subscription.as_mut().unwrap();let mut messages=sub.on_message();
                tokio::select!{biased;
                    _=shutdown.changed()=>break,
                    _=interval.tick()=>{if self.renew().await.is_err()||self.restore().await.is_err(){self.ready.store(false,Ordering::Release);}},
                    message=messages.next()=>match message{Some(message)=>{if self.healthy()&& let Ok(packet)=message.get_payload::<String>().and_then(|s|serde_json::from_str::<Packet>(&s).map_err(|_|redis::RedisError::from((redis::ErrorKind::TypeError,"invalid packet")))){self.deliver(packet);}},None=>{self.ready.store(false,Ordering::Release);}}
                }
            }

            self.ready.store(false,Ordering::Release);
        });
    }

    pub(super) async fn restore(&self) -> SocketResult<()> {
        let _membership = self.membership.lock().await;
        let Some(shared) = self.state.upgrade() else {
            return Ok(());
        };
        let rooms = {
            let state = shared.lock().unwrap();

            state
                .rooms
                .iter()
                .map(|((path, room), value)| {
                    (
                        path.clone(),
                        room.clone(),
                        value.members.clone(),
                        value.generation.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };

        for (path, room, members, generation) in rooms {
            let info = async {
                let value = self.execute(&path, &room, "info", &[]).await?;
                let fields: Vec<String> = redis::from_redis_value(&value).map_err(unavailable)?;

                Ok::<_, SocketError>(RoomInfo {
                    mode: serde_json::from_str(&fields[0])?,
                    generation: fields[1].clone(),
                    high: fields[2].clone(),
                })
            }
            .await;
            #[cfg(test)]
            self.checkpoint("restore_info").await;
            let valid = match &info {
                Ok(info) => Some(info.generation.as_str()) == generation.as_deref(),
                Err(error) if error.code() == "room_not_found" => false,
                Err(_) => return info.map(|_| ()),
            };

            if !valid {
                shared
                    .lock()
                    .unwrap()
                    .retire_generation(&path, &room, generation.as_deref());
                continue;
            }

            for member in members {
                match self
                    .execute(
                        &path,
                        &room,
                        "renew_member",
                        &[member.to_string(), generation.clone().unwrap_or_default()],
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(e) if matches!(e.code(), "room_not_found" | "history_expired") => {
                        shared.lock().unwrap().retire_generation(
                            &path,
                            &room,
                            generation.as_deref(),
                        );
                        break;
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(())
    }

    fn deliver(&self, packet: Packet) {
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let state = state.lock().unwrap();
        let message = match packet.kind.as_str() {
            "text" => Message::Text(packet.text.unwrap_or_default().into()),
            "binary" => Message::Binary(packet.binary.unwrap_or_default().into()),
            _ => return,
        };
        let senders = match packet.room {
            Some(room) => state
                .rooms
                .get(&(packet.path.clone(), room))
                .filter(|r| r.generation.as_deref() == packet.generation.as_deref())
                .map(|r| {
                    r.members
                        .iter()
                        .filter_map(|id| state.connections.get(&(packet.path.clone(), id.clone())))
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            None => state
                .connections
                .iter()
                .filter(|((path, _), _)| path == &packet.path)
                .map(|(_, s)| s.clone())
                .collect(),
        };

        for sender in senders {
            let _ = sender.send(message.clone());
        }
    }
}

pub(crate) struct RoomInfo {
    pub mode: RoomMode,
    pub generation: String,
    pub high: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Packet {
    #[serde(default)]
    generation: Option<String>,
    path: String,
    room: Option<String>,
    kind: String,
    text: Option<String>,
    binary: Option<Vec<u8>>,
}

impl Packet {
    fn new(path: &str, room: Option<&str>, message: Message) -> SocketResult<Self> {
        let (kind, text, binary) = match message {
            Message::Text(v) => ("text", Some(v.to_string()), None),
            Message::Binary(v) => ("binary", None, Some(v.to_vec())),
            _ => return Err(SocketError::new("invalid_message")),
        };

        Ok(Self {
            generation: None,
            path: path.to_owned(),
            room: room.map(str::to_owned),
            kind: kind.into(),
            text,
            binary,
        })
    }
}

fn ensure_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn redis_tls_initializes_a_crypto_provider_without_native_socket_tls() {
        super::ensure_crypto_provider();
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }
}
