//! Independent per-membership cursors preserve stream order across reconnects.
use crate::{SocketError, SocketResult};

pub(crate) fn parse_id(id: &str) -> SocketResult<(u64, u64)> {
    let (a, b) = id
        .split_once('-')
        .ok_or_else(|| SocketError::new("invalid_cursor"))?;

    if a.is_empty() || b.is_empty() || !a.bytes().chain(b.bytes()).all(|b| b.is_ascii_digit()) {
        return Err(SocketError::new("invalid_cursor"));
    }

    Ok((
        a.parse().map_err(|_| SocketError::new("invalid_cursor"))?,
        b.parse().map_err(|_| SocketError::new("invalid_cursor"))?,
    ))
}

use super::redis::{RedisBackend, RoomInfo, unavailable};
use crate::{Message, SocketContext, SocketEnvelope};
use std::sync::Arc;

async fn page(
    backend: &RedisBackend,
    context: &SocketContext,
    room: &str,
    generation: &str,
    cursor: &mut String,
    high: &str,
    stopped: &tokio::sync::watch::Receiver<bool>,
) -> SocketResult<bool> {
    let result = backend
        .command(
            &context.hub.path,
            room,
            "read",
            &[generation.to_owned(), cursor.clone(), high.to_owned()],
        )
        .await?;
    let entries: Vec<(String, Vec<String>)> =
        redis::from_redis_value(&result).map_err(unavailable)?;
    let any = !entries.is_empty();

    for (id, fields) in entries {
        if fields.len() != 2 || fields[0] != "event" {
            return Err(SocketError::new("internal_error"));
        }

        let mut envelope = SocketEnvelope::parse(&fields[1])?;
        envelope.cursor = Some(id.clone());
        envelope.id = None;
        let state = context.hub.hub.state.lock().unwrap();

        if *stopped.borrow()
            || !state
                .rooms
                .get(&(context.hub.path.clone(), room.to_owned()))
                .is_some_and(|r| {
                    r.generation.as_deref() == Some(generation) && r.members.contains(context.id())
                })
        {
            return Err(SocketError::new("closed"));
        }

        state
            .connections
            .get(&(context.hub.path.clone(), context.id().clone()))
            .ok_or_else(|| SocketError::new("closed"))?
            .send(Message::Text(envelope.to_text()?.into()))?;
        *cursor = id;
    }

    Ok(any)
}

pub(crate) async fn join(
    backend: Arc<RedisBackend>,
    context: SocketContext,
    room: String,
    after: Option<String>,
    info: RoomInfo,
) -> SocketResult<()> {
    let (stop, mut stopped) = tokio::sync::watch::channel(false);

    if let Some(old) = context.hub.hub.state.lock().unwrap().readers.insert(
        (context.hub.path.clone(), room.clone(), context.id().clone()),
        stop,
    ) {
        old.send_replace(true);
    }

    let mut cursor = after.unwrap_or_else(|| info.high.clone());
    #[cfg(test)]
    backend.checkpoint("replay_join").await;

    while parse_id(&cursor)? < parse_id(&info.high)? {
        if !page(
            &backend,
            &context,
            &room,
            &info.generation,
            &mut cursor,
            &info.high,
            &stopped,
        )
        .await?
        {
            cursor = info.high.clone();
            break;
        }
    }

    if *stopped.borrow() {
        return Err(SocketError::new("closed"));
    }

    context.acknowledge_join(&room).await?;
    let mut context = context;
    context.request_id = None;
    let tasks = context.tasks.clone();
    let mut shutdown = context.shutdown.clone();
    tasks.spawn(async move {
        loop {
            if *stopped.borrow() || *shutdown.borrow() {
                break;
            }

            let reading = stopped.clone();
            let read = async {
                if !backend.healthy() {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

                    return Ok(());
                }

                {
                    let _transition = context.transitions.lock().await;

                    if *reading.borrow() {
                        return Err(SocketError::new("closed"));
                    }

                    if page(
                        &backend,
                        &context,
                        &room,
                        &info.generation,
                        &mut cursor,
                        "+",
                        &reading,
                    )
                    .await?
                    {
                        return Ok(());
                    }
                }
                let mut connection = backend
                    .client
                    .get_multiplexed_async_connection()
                    .await
                    .map_err(unavailable)?;
                let _: redis::Value = redis::cmd("XREAD")
                    .arg("COUNT")
                    .arg(64)
                    .arg("BLOCK")
                    .arg(200)
                    .arg("STREAMS")
                    .arg(&backend.keys(&context.hub.path, &room)[2])
                    .arg(&cursor)
                    .query_async(&mut connection)
                    .await
                    .map_err(unavailable)?;

                Ok::<(), SocketError>(())
            };
            tokio::select! {
                biased;
                _ = shutdown.changed() => break,
                _ = stopped.changed() => break,
                result = read => {
                    if let Err(error) = result {
                        if error.code() == "closed" {
                            break;
                        }
                        if error.code() != "backend_unavailable" {
                            let _transition = context.transitions.lock().await;
                            if !*stopped.borrow() {
                                let _ = crate::dispatch::send_room_error(&context, error, Some(&room)).await;
                            }
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                }
            }
        }
    });

    Ok(())
}
