//! Definition of chat abstraction.

use std::collections::BTreeSet;

use crate::server::chat_event::ChatEvent;

/// Abstraction on chat.
/// Prevents chat events reordering by tracking event sequence numbers.
struct Chat {
    name: String,
    /// Sequence number of next chat event.
    seq: usize,
    /// Sorted by sequence number
    pending_events: BTreeSet<ChatEvent>,
}

impl Chat {
    pub fn new(name: String) -> Self {
        Self {
            name,
            seq: 0,
            pending_events: BTreeSet::new(),
        }
    }
}
