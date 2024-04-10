use core::fmt;
///! Definition of messages which can be produces by server.
use std::time::SystemTime;

use chrono::{DateTime, Local};
use dsbuild::Message;
use serde::{Deserialize, Serialize};

use super::chat_event::ChatEvent;

/// Represents response from server to client request.
pub type RequestResponse = Result<(), String>;

/// Represents messages from server to the client.
///
/// Server can send even ack/nack of client requests,
/// or events which were appeared in the client's chat.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ServerMessageKind {
    RequestResponse(usize, RequestResponse), // Response on request.
    ChatEvents(String, Vec<ChatEvent>),      // Name of the chat and chat events.
}

impl fmt::Display for ServerMessageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerMessageKind::RequestResponse(id, response) => {
                write!(f, "[id={}] response={:?}", id, response)
            }
            ServerMessageKind::ChatEvents(chat, events) => {
                let mut events_string = vec!["[\n".to_owned()];
                for event in events {
                    events_string.push("\t".to_owned());
                    let ser = event.to_string();
                    events_string.push(ser);
                    events_string.push("\n".to_owned());
                }
                events_string.push("]".to_owned());
                write!(f, "chat={}, events={}", chat, events_string.join(""))
            }
        }
    }
}

/// Represents messages from server to the client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMessage {
    pub server: String,
    pub time: SystemTime,
    pub kind: ServerMessageKind,
}

impl fmt::Display for ServerMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dt: DateTime<Local> = self.time.clone().into();

        write!(f, "[{}]\t {}", dt.format("%Y-%m-%d %H:%M:%S"), self.kind)
    }
}

impl From<ServerMessage> for Message {
    fn from(value: ServerMessage) -> Self {
        Message::borrow_new("SERVER_MESSAGE", value).unwrap()
    }
}

pub struct ServerMessageBuilder {
    server: String,
}

impl ServerMessageBuilder {
    pub fn new(server: String) -> Self {
        Self { server }
    }

    pub fn new_with_kind(&self, kind: ServerMessageKind) -> ServerMessage {
        ServerMessage {
            server: self.server.clone(),
            time: SystemTime::now(),
            kind,
        }
    }

    pub fn new_chat_event(&self, event: ChatEvent) -> ServerMessage {
        self.new_with_kind(ServerMessageKind::ChatEvents(
            event.chat.clone(),
            vec![event],
        ))
    }

    pub fn new_chat_events(&self, chat: String, events: Vec<ChatEvent>) -> ServerMessage {
        self.new_with_kind(ServerMessageKind::ChatEvents(chat, events))
    }

    pub fn new_good_response(&self, request_id: usize) -> ServerMessage {
        self.new_with_kind(ServerMessageKind::RequestResponse(request_id, Ok(())))
    }

    pub fn new_bad_response(&self, request_id: usize, error: String) -> ServerMessage {
        self.new_with_kind(ServerMessageKind::RequestResponse(request_id, Err(error)))
    }
}
