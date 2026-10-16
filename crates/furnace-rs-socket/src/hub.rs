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
