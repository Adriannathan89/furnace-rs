//! Dynamic room definitions shared by local and Redis delivery.
use crate::ConnectionId;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Duration};
/// Delivery semantics fixed for the lifetime of a room.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RoomMode {
    /// Live messages with no history or replay.
    Live,
    /// Named events retained up to both explicit limits.
    Replayable {
        /// Maximum retained event age.
        max_age: Duration,
        /// Maximum retained event count.
        max_messages: usize,
    },
}
pub(crate) struct Room {
    pub mode: RoomMode,
    pub members: BTreeSet<ConnectionId>,
    #[cfg_attr(not(feature = "redis"), allow(dead_code))]
    pub generation: Option<String>,
}
#[cfg(feature = "redis")]
pub(crate) mod redis;
#[cfg(feature = "redis")]
pub(crate) mod replay;
