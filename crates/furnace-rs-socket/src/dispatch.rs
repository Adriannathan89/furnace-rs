//! Bounded managed gateway execution independent of WebSocket control frames.
use crate::runtime::SocketRuntime;
use crate::{
    ConnectionId, Message, ProcessingMode, SocketContext, SocketEndpointDescriptor, SocketEnvelope,
    SocketError, SocketHandshake, SocketResult, WebSocket,
};
use futures_util::{SinkExt, StreamExt, future::BoxFuture};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{task::JoinSet, time::Instant};

/// Input delivered by generated gateway adapters.
#[doc(hidden)]
pub enum SocketInput {
    Event(String, serde_json::Value),
    Text(String),
    Binary(axum::body::Bytes),
    Connect,
    Disconnect,
    Join(String),
}

/// Typed gateway adapter emitted by the registration macro.
#[doc(hidden)]
pub trait SocketGateway: Send + Sync + 'static {
    fn call(&self, context: SocketContext, input: SocketInput) -> BoxFuture<'_, SocketResult<()>>;
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

struct Cleanup {
    hub: crate::SocketGatewayHub,
    id: ConnectionId,
    tasks: furnace_rs_common::runtime::RuntimeTasks,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let (rooms, backend) = {
            let mut state = self.hub.hub.state.lock().expect("socket hub poisoned");
            state
                .connections
                .remove(&(self.hub.path.clone(), self.id.clone()));
            let mut rooms = Vec::new();

            for ((path, name), room) in &mut state.rooms {
                if path == &self.hub.path && room.members.remove(&self.id) {
                    rooms.push(name.clone());
                }
            }

            state.readers.retain(|(_, _, id), stop| {
                if id == &self.id {
                    stop.send_replace(true);

                    false
                } else {
                    true
                }
            });
            #[cfg(feature = "redis")]
            let backend = state.backend.clone();
            #[cfg(not(feature = "redis"))]
            let backend: Option<()> = None;

            (rooms, backend)
        };

        #[cfg(feature = "redis")]
        if let Some(backend) = backend {
            let path = self.hub.path.clone();
            let id = self.id.clone();
            self.tasks.spawn(async move {
                let _membership = backend.membership.lock().await;

                for room in rooms {
                    let _ = backend.leave(&path, &room, &id).await;
                }
            });
        }

        #[cfg(not(feature = "redis"))]
        let _ = (rooms, backend, &self.tasks);
    }
}

pub(crate) async fn run(
    gateway: Arc<dyn SocketGateway>,
    endpoint: &'static SocketEndpointDescriptor,
    handshake: SocketHandshake,
    socket: WebSocket,
    mut runtime: SocketRuntime,
) {
    let id = ConnectionId(format!(
        "{}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let hub = runtime.hub.gateway(endpoint.path);
    let (sender, mut outbound, mut closed) = crate::hub::sender(&runtime.config, endpoint.mode);
    hub.hub
        .state
        .lock()
        .expect("socket hub poisoned")
        .connections
        .insert((hub.path.clone(), id.clone()), sender.clone());
    let _cleanup = Cleanup {
        hub: hub.clone(),
        id: id.clone(),
        tasks: runtime.tasks.clone(),
    };
    let context = SocketContext {
        transitions: Arc::new(tokio::sync::Mutex::new(())),
        id,
        handshake,
        hub,
        acknowledge_join: false,
        request_id: None,
        gateway: gateway.clone(),
        #[cfg(feature = "redis")]
        tasks: runtime.tasks.clone(),
        #[cfg(feature = "redis")]
        shutdown: runtime.shutdown.clone(),
    };
    let (mut sink, mut stream) = socket.split();
    let mut writer_stop = runtime.shutdown.clone();
    let mut writer_closed = closed.clone();
    let tasks = runtime.tasks.clone();
    let close_timeout = runtime.config.shutdown_timeout;
    tasks.spawn(async move {
        loop {
            tokio::select!{biased;
                _=writer_stop.changed()=>{let _=tokio::time::timeout(close_timeout,sink.send(Message::Close(None))).await;break;},
                _=writer_closed.changed()=>{let _=tokio::time::timeout(close_timeout,sink.send(Message::Close(None))).await;break;},
                queued=outbound.recv()=>match queued {
                    Some(queued)=>{
                        tokio::select!{biased;
                            _=writer_stop.changed()=>break,
                            _=writer_closed.changed()=>break,
                            result=sink.send(queued.message)=>if result.is_err(){break;}
                        }
                    },
                    None=>break,
                }
            }
        }
    });
    let mut connecting = Some(gateway.call(context.clone(), SocketInput::Connect));
    let limit = match endpoint.processing {
        ProcessingMode::Sequential => 1,
        ProcessingMode::Concurrent { max_concurrency } => max_concurrency,
    };
    let mut pending: VecDeque<(SocketContext, SocketInput, usize)> = VecDeque::new();
    let mut pending_bytes = 0usize;
    let mut jobs = JoinSet::new();
    let mut next_ping = Instant::now() + runtime.config.heartbeat_interval;
    let mut awaiting: Option<(Vec<u8>, Instant)> = None;

    loop {
        while connecting.is_none() && jobs.len() < limit {
            if let Some((ctx, input, size)) = pending.pop_front() {
                pending_bytes -= size;
                let gateway = gateway.clone();
                jobs.spawn(async move {
                    let result = handle(gateway, ctx.clone(), input).await;

                    if let Err(error) = result {
                        let _ = send_error(&ctx, error).await;
                    }
                });
            } else {
                break;
            }
        }

        if *runtime.shutdown.borrow() || *closed.borrow() {
            break;
        }

        let pong_deadline = awaiting
            .as_ref()
            .map(|(_, at)| *at)
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(86_400));
        tokio::select! {biased;
            _=runtime.shutdown.changed()=>break,
            _=closed.changed()=>break,
            _=tokio::time::sleep_until(pong_deadline),if awaiting.is_some()=>break,
            _=tokio::time::sleep_until(next_ping),if awaiting.is_none()=>{
                let nonce=NEXT_ID.fetch_add(1,Ordering::Relaxed).to_be_bytes().to_vec();
                if sender.send(Message::Ping(nonce.clone().into())).is_err(){break;}
                awaiting=Some((nonce,Instant::now()+runtime.config.pong_timeout));
            },
            result=async {connecting.as_mut().unwrap().await},if connecting.is_some()=>{
                connecting=None;
                if result.is_err(){break;}
            },
            _=jobs.join_next(),if !jobs.is_empty()=>{},
            incoming=stream.next()=>{
                let message=match incoming{Some(Ok(message))=>message,_=>break};let size=crate::hub::message_size(&message);
                let mut ctx=context.clone();
                let input=match message {
                    Message::Close(_)=>break,
                    Message::Pong(data)=>{if awaiting.as_ref().is_some_and(|(nonce,_)|nonce.as_slice()==data.as_ref()){awaiting=None;next_ping=Instant::now()+runtime.config.heartbeat_interval;}continue;},
                    Message::Ping(data)=>{let _=sender.send(Message::Pong(data));continue;},
                    Message::Text(text) if endpoint.mode==crate::EndpointMode::Events=>match SocketEnvelope::parse(&text){Ok(event)=>{ctx.request_id=event.id;if event.room.is_some()||event.cursor.is_some(){let _=send_error(&ctx,SocketError::new("invalid_message")).await;continue;}SocketInput::Event(event.event,event.data)},Err(error)=>{let _=send_error(&ctx,error).await;continue;}},
                    Message::Text(text)=>SocketInput::Text(text.to_string()),
                    Message::Binary(data) if endpoint.mode==crate::EndpointMode::Raw=>SocketInput::Binary(data),
                    _=>{let _=send_error(&ctx,SocketError::new("invalid_message")).await;continue;}
                };
                if pending.len()>=runtime.config.queue_messages || pending_bytes.saturating_add(size)>runtime.config.queue_bytes {break;}
                pending_bytes+=size;pending.push_back((ctx,input,size));
            }
        }
    }

    drop(connecting);
    jobs.abort_all();

    while jobs.join_next().await.is_some() {}

    sender.close.send_replace(true);
    let _ = gateway.call(context, SocketInput::Disconnect).await;
}

async fn send_error(context: &SocketContext, error: SocketError) -> SocketResult<()> {
    send_room_error(context, error, None).await
}

pub(crate) async fn send_room_error(
    context: &SocketContext,
    error: SocketError,
    room: Option<&str>,
) -> SocketResult<()> {
    let mut envelope = SocketEnvelope::event(
        "furnace.error",
        serde_json::json!({"code":error.code(),"message":if error.code()=="internal_error"{"internal server error"}else{error.code()}}),
    );
    envelope.id = context.request_id.clone();

    if let Some(room) = room {
        envelope.room = Some(room.to_owned());
        envelope.data["room"] = serde_json::Value::String(room.to_owned());
    }

    let state = context.hub.hub.state.lock().expect("socket hub poisoned");

    state
        .connections
        .get(&(context.hub.path.clone(), context.id.clone()))
        .ok_or_else(|| SocketError::new("closed"))?
        .error(envelope)
}

async fn handle(
    gateway: Arc<dyn SocketGateway>,
    context: SocketContext,
    input: SocketInput,
) -> SocketResult<()> {
    if let SocketInput::Event(name, data) = &input
        && name.starts_with("furnace.")
    {
        let room = data
            .get("room")
            .and_then(serde_json::Value::as_str)
            .filter(|r| !r.is_empty())
            .ok_or_else(|| SocketError::new("invalid_message"))?;

        match name.as_str() {
            "furnace.join" => {
                let after = match data.get("after") {
                    Some(serde_json::Value::String(v)) => Some(v.as_str()),
                    None => None,
                    _ => return Err(SocketError::new("invalid_cursor")),
                };
                let mut context = context.clone();
                context.acknowledge_join = true;

                return context.join(room, after).await;
            }
            "furnace.leave" => context.leave(room).await?,
            _ => return Err(SocketError::new("unknown_event")),
        }

        return context
            .emit("furnace.ok", serde_json::json!({"room":room}))
            .await;
    }

    gateway.call(context, input).await
}
