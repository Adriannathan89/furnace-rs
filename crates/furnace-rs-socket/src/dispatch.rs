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
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        self.hub
            .hub
            .state
            .lock()
            .expect("socket hub poisoned")
            .connections
            .remove(&(self.hub.path.clone(), self.id.clone()));
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
    let (sender, mut outbound, mut closed) = crate::hub::sender(&runtime.config);
    hub.hub
        .state
        .lock()
        .expect("socket hub poisoned")
        .connections
        .insert((hub.path.clone(), id.clone()), sender.clone());
    let _cleanup = Cleanup {
        hub: hub.clone(),
        id: id.clone(),
    };
    let context = SocketContext {
        id,
        handshake,
        hub,
        request_id: None,
    };
    let (mut sink, mut stream) = socket.split();
    let mut writer_stop = runtime.shutdown.clone();
    let mut writer_closed = closed.clone();
    let tasks = runtime.tasks.clone();
    tasks.spawn(async move {
        loop {tokio::select!{biased;
            _=writer_stop.changed()=>{let _=sink.send(Message::Close(None)).await;break;},
            _=writer_closed.changed()=>{let _=sink.send(Message::Close(None)).await;break;},
            queued=outbound.recv()=>match queued{Some(queued)=>{if sink.send(queued.message).await.is_err(){break;}},None=>break}
        }}
    });
    let connect = gateway.call(context.clone(), SocketInput::Connect);
    tokio::pin!(connect);
    tokio::select! {_=runtime.shutdown.changed()=>{sender.close.send_replace(true);return;},result=&mut connect=>if result.is_err(){sender.close.send_replace(true);return;}}
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
        while jobs.len() < limit {
            if let Some((ctx, input, size)) = pending.pop_front() {
                pending_bytes -= size;
                let gateway = gateway.clone();
                jobs.spawn(async move {
                    let result = gateway.call(ctx.clone(), input).await;
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
    jobs.abort_all();
    while jobs.join_next().await.is_some() {}
    sender.close.send_replace(true);
    let _ = gateway.call(context, SocketInput::Disconnect).await;
}
async fn send_error(context: &SocketContext, error: SocketError) -> SocketResult<()> {
    context.emit("furnace.error",serde_json::json!({"code":error.code(),"message":if error.code()=="internal_error"{"internal server error"}else{error.code()}})).await
}
