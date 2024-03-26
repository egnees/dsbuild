use std::collections::HashSet;

use super::messages::{ChatEvent, ChatEventKind};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Chat {
    name: String,
    connected_users: HashSet<String>,
    history: Vec<ChatEvent>,
}

impl Chat {
    pub fn new(name: String, initiator: String) -> (Self, ChatEvent) {
        let mut chat = Self {
            name,
            connected_users: HashSet::new(),
            history: Vec::new(),
        };

        let event = chat.make_chat_event(initiator, ChatEventKind::Created());

        (chat, event)
    }

    fn make_chat_event(&mut self, user: String, kind: ChatEventKind) -> ChatEvent {
        let event = ChatEvent::new_with_kind(self.name.clone(), user, kind);
        self.history.push(event.clone());
        event
    }

    pub fn is_connected(&self, user: &str) -> bool {
        self.connected_users.contains(user)
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns event and history with connect event.
    pub fn connect_user(&mut self, name: String) -> (ChatEvent, Vec<ChatEvent>) {
        let result = self.connected_users.insert(name.clone());
        assert!(result, "trying to connect connected user");

        let event = self.make_chat_event(name, ChatEventKind::Connected());

        (event, self.history.clone())
    }

    pub fn disconnect_user(&mut self, name: String) -> ChatEvent {
        let result = self.connected_users.remove(name.as_str());
        assert!(result, "trying to disconnect not connected user");

        self.make_chat_event(name, ChatEventKind::Disconnected())
    }

    pub fn send_message(&mut self, user: String, message: String) -> ChatEvent {
        assert!(self.connected_users.contains(user.as_str()));

        self.make_chat_event(user, ChatEventKind::SentMessage(message))
    }

    pub fn connected_users(&self) -> Vec<String> {
        self.connected_users.clone().into_iter().collect()
    }
}
