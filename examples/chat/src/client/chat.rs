//! Definition of chat abstraction.

use std::collections::BTreeSet;

use crate::server::event::ChatEvent;

/// Abstraction on chat.
/// Prevents chat events reordering by tracking event sequence numbers.
#[derive(Debug, Clone)]
pub struct Chat {
    name: String,
    /// Sequence number of next chat event.
    seq: u64,
    /// Sorted by sequence number.
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

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Process arrived chat event.
    /// Returns events in chat, which became visible to user.
    pub fn process_event(&mut self, event: ChatEvent) -> Vec<ChatEvent> {
        assert_eq!(
            self.name,
            event.chat,
            "bad event chat name, expected '{}'",
            self.name.as_str()
        );
        if event.seq < self.seq {
            Vec::new()
        } else {
            self.pending_events.insert(event);
            self.extract_ready_events()
        }
    }

    /// Accepts not ordered sequence of chat events.
    /// Returns events which became visible to user.
    pub fn process_events(&mut self, events: Vec<ChatEvent>) -> Vec<ChatEvent> {
        for event in events.into_iter() {
            if event.seq >= self.seq {
                self.pending_events.insert(event);
            }
        }

        self.extract_ready_events()
    }

    fn extract_ready_events(&mut self) -> Vec<ChatEvent> {
        let mut result = Vec::new();
        while let Some(first_event) = self.pending_events.first() {
            if first_event.seq > self.seq {
                break;
            }

            assert!(first_event.seq == self.seq);

            let event = self.pending_events.pop_first().unwrap();
            result.push(event);

            self.seq += 1;
        }

        result
    }
}
