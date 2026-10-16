//! Cloneable application sending interface and bounded connection registration.
use crate::{ConnectionId, Message, SocketError, SocketResult, SocketServerConfig};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch};
pub(crate) struct Queued {
    pub message: Message,
    pub _permit: OwnedSemaphorePermit,
}
#[derive(Clone)]
pub(crate) struct Sender {
    pub queue: mpsc::Sender<Queued>,
    pub bytes: Arc<Semaphore>,
    pub close: watch::Sender<bool>,
}
impl Sender {
    pub fn send(&self, message: Message) -> SocketResult<()> {
        let size = message_size(&message).max(1);
        let permit = u32::try_from(size)
            .ok()
            .and_then(|n| self.bytes.clone().try_acquire_many_owned(n).ok());
        let result = match permit {
            Some(permit) => self
                .queue
                .try_send(Queued {
                    message,
                    _permit: permit,
                })
                .map_err(|_| SocketError::new("slow_consumer")),
            None => Err(SocketError::new("slow_consumer")),
        };
        if result.is_err() {
            self.close.send_replace(true);
        }
        result
    }
}
#[derive(Default)]
pub(crate) struct HubState {
    pub connections: BTreeMap<(String, ConnectionId), Sender>,
    pub rooms: BTreeMap<(String, String), crate::rooms::Room>,
    #[cfg(feature = "redis")]
    pub backend: Option<Arc<crate::rooms::redis::RedisBackend>>,
    pub readers: BTreeMap<(String, String, ConnectionId), watch::Sender<bool>>,
}
/// Application-owned sending interface; register as a provider to inject into services.
#[derive(Clone, Default)]
pub struct SocketHub {
    pub(crate) state: Arc<Mutex<HubState>>,
}
impl SocketHub {
    /// Scopes room operations and broadcasts to one gateway path.
    pub fn gateway(&self, path: &str) -> SocketGatewayHub {
        SocketGatewayHub {
            hub: self.clone(),
            path: path.to_owned(),
        }
    }
}
impl furnace_rs_core::Injector for SocketHub {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self::default())
    }
}
/// Gateway-local view of the application sending interface.
#[derive(Clone)]
pub struct SocketGatewayHub {
    pub(crate) hub: SocketHub,
    pub(crate) path: String,
}
impl SocketGatewayHub {
    /// Sends directly to a connection on this instance.
    pub async fn send(&self, id: &ConnectionId, message: Message) -> SocketResult<()> {
        let sender = self
            .hub
            .state
            .lock()
            .expect("socket hub poisoned")
            .connections
            .get(&(self.path.clone(), id.clone()))
            .cloned()
            .ok_or_else(|| SocketError::new("closed"))?;
        sender.send(message)
    }
    /// Sends a live message to every managed connection on this gateway.
    pub async fn broadcast(&self, message: Message) -> SocketResult<()> {
        #[cfg(feature = "redis")]
        if let Some(backend) = self.backend() {
            return backend.publish(&self.path, None, message).await;
        }

        let senders = self
            .hub
            .state
            .lock()
            .expect("socket hub poisoned")
            .connections
            .iter()
            .filter(|((path, _), _)| path == &self.path)
            .map(|(_, sender)| sender.clone())
            .collect::<Vec<_>>();
        for sender in senders {
            let _ = sender.send(message.clone());
        }
        Ok(())
    }
}
pub(crate) fn sender(
    config: &SocketServerConfig,
) -> (Sender, mpsc::Receiver<Queued>, watch::Receiver<bool>) {
    let (queue, receiver) = mpsc::channel(config.queue_messages);
    let (close, closed) = watch::channel(false);
    (
        Sender {
            queue,
            bytes: Arc::new(Semaphore::new(config.queue_bytes)),
            close,
        },
        receiver,
        closed,
    )
}

pub(crate) fn message_size(message: &Message) -> usize {
    match message {
        Message::Text(v) => v.len(),
        Message::Binary(v) | Message::Ping(v) | Message::Pong(v) => v.len(),
        Message::Close(v) => v.as_ref().map_or(2, |v| v.reason.len() + 2),
    }
}
impl SocketGatewayHub {
    /// Creates a dynamic room; identical definitions are idempotent.
    pub async fn create_room(&self, room: &str, mode: crate::RoomMode) -> SocketResult<()> {
        if room.is_empty() {
            return Err(SocketError::new("invalid_room"));
        }
        if let crate::RoomMode::Replayable {
            max_age,
            max_messages,
        } = &mode
            && (max_age.is_zero() || *max_messages == 0)
        {
            return Err(SocketError::new("invalid_retention"));
        }
        #[cfg(feature = "redis")]
        if let Some(backend) = self.backend() {
            backend.create(&self.path, room, &mode).await?;
            let info = backend.info(&self.path, room).await?;
            self.hub
                .state
                .lock()
                .unwrap()
                .rooms
                .entry((self.path.clone(), room.to_owned()))
                .or_insert(crate::rooms::Room {
                    mode,
                    members: Default::default(),
                    generation: Some(info.generation),
                });
            return Ok(());
        }
        if !matches!(mode, crate::RoomMode::Live) {
            return Err(SocketError::new("backend_unavailable"));
        }

        let mut state = self.hub.state.lock().expect("socket hub poisoned");
        let key = (self.path.clone(), room.to_owned());
        if let Some(existing) = state.rooms.get(&key) {
            return if existing.mode == mode {
                Ok(())
            } else {
                Err(SocketError::new("room_conflict"))
            };
        }
        state.rooms.insert(
            key,
            crate::rooms::Room {
                mode,
                members: Default::default(),
                generation: None,
            },
        );
        Ok(())
    }
    /// Deletes a room only when it has no active members.
    pub async fn delete_room(&self, room: &str) -> SocketResult<()> {
        #[cfg(feature = "redis")]
        if let Some(backend) = self.backend() {
            backend.delete(&self.path, room).await?;
            self.hub
                .state
                .lock()
                .unwrap()
                .rooms
                .remove(&(self.path.clone(), room.to_owned()));
            return Ok(());
        }

        let mut state = self.hub.state.lock().expect("socket hub poisoned");
        let key = (self.path.clone(), room.to_owned());
        let room = state
            .rooms
            .get(&key)
            .ok_or_else(|| SocketError::new("room_not_found"))?;
        if !room.members.is_empty() {
            return Err(SocketError::new("room_not_empty"));
        }
        state.rooms.remove(&key);
        Ok(())
    }
    /// Sends a live native message to current room members.
    pub async fn send_room(&self, room: &str, message: Message) -> SocketResult<()> {
        #[cfg(feature = "redis")]
        if let Some(backend) = self.backend() {
            return backend.publish(&self.path, Some(room), message).await;
        }

        let senders = {
            let state = self.hub.state.lock().expect("socket hub poisoned");
            let room = state
                .rooms
                .get(&(self.path.clone(), room.to_owned()))
                .ok_or_else(|| SocketError::new("room_not_found"))?;
            if !matches!(room.mode, crate::RoomMode::Live) {
                return Err(SocketError::new("invalid_message"));
            }
            room.members
                .iter()
                .filter_map(|id| {
                    state
                        .connections
                        .get(&(self.path.clone(), id.clone()))
                        .cloned()
                })
                .collect::<Vec<_>>()
        };
        for sender in senders {
            let _ = sender.send(message.clone());
        }
        Ok(())
    }
    /// Emits a named room event; replayable delivery returns its cursor.
    pub async fn emit_room(
        &self,
        room: &str,
        event: &str,
        data: serde_json::Value,
    ) -> SocketResult<Option<String>> {
        if event.is_empty() || event.starts_with("furnace.") {
            return Err(SocketError::new("invalid_message"));
        }
        let mut envelope = crate::SocketEnvelope::event(event, data);
        envelope.room = Some(room.to_owned());
        #[cfg(feature = "redis")]
        if let Some(backend) = self.backend()
            && matches!(
                backend.info(&self.path, room).await?.mode,
                crate::RoomMode::Replayable { .. }
            )
        {
            return backend.append(&self.path, room, envelope).await.map(Some);
        }
        self.send_room(room, Message::Text(envelope.to_text()?.into()))
            .await?;
        Ok(None)
    }
    pub(crate) async fn join(
        &self,
        context: &crate::SocketContext,
        room: &str,
        after: Option<&str>,
    ) -> SocketResult<()> {
        #[cfg(feature = "redis")]
        if let Some(backend) = self.backend() {
            let info = backend.join(&self.path, room, context.id(), after).await?;
            {
                let mut state = self.hub.state.lock().unwrap();
                let entry = state
                    .rooms
                    .entry((self.path.clone(), room.to_owned()))
                    .or_insert(crate::rooms::Room {
                        mode: info.mode.clone(),
                        members: Default::default(),
                        generation: Some(info.generation.clone()),
                    });
                entry.generation = Some(info.generation.clone());
                entry.members.insert(context.id().clone());
            }
            let result = if matches!(info.mode, crate::RoomMode::Replayable { .. }) {
                crate::rooms::replay::join(
                    backend,
                    context.clone(),
                    room.to_owned(),
                    after.map(str::to_owned),
                    info,
                )
                .await
            } else {
                context.acknowledge_join(room).await
            };
            if result.is_err() {
                let _ = self.leave(context.id(), room).await;
            }
            return result;
        }
        if after.is_some() {
            return Err(SocketError::new("invalid_cursor"));
        }
        {
            let mut state = self.hub.state.lock().expect("socket hub poisoned");
            state
                .rooms
                .get_mut(&(self.path.clone(), room.to_owned()))
                .ok_or_else(|| SocketError::new("room_not_found"))?
                .members
                .insert(context.id().clone());
        }
        context.acknowledge_join(room).await
    }
    pub(crate) async fn leave(&self, id: &ConnectionId, room: &str) -> SocketResult<()> {
        #[cfg(feature = "redis")]
        if let Some(backend) = self.backend() {
            backend.leave(&self.path, room, id).await?;
        }

        let mut state = self.hub.state.lock().expect("socket hub poisoned");
        if let Some(stop) = state
            .readers
            .remove(&(self.path.clone(), room.to_owned(), id.clone()))
        {
            stop.send_replace(true);
        }
        if let Some(room) = state.rooms.get_mut(&(self.path.clone(), room.to_owned())) {
            room.members.remove(id);
        }
        Ok(())
    }
}

impl SocketGatewayHub {
    #[cfg(feature = "redis")]
    pub(crate) fn backend(&self) -> Option<Arc<crate::rooms::redis::RedisBackend>> {
        self.hub.state.lock().unwrap().backend.clone()
    }
}
