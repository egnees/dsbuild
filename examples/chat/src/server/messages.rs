use core::fmt;
///! Definition of messages which can be produces by server.
use std::time::SystemTime;

use chrono::DateTime;
use chrono::Local;
use colored::Colorize;

use dsbuild::Message;
use serde::{Deserialize, Serialize};

/// Represents response from server to client request.
pub type RequestResponse = Result<(), String>;

/// Represents event in the chat.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ChatEventKind {
    SentMessage(String), // Client sent message,
    Connected(),         // Client connected to server,
    Disconnected(),      // Client disconnected from server,
    Created(),           // Client created chat.
}

/// Represents chat event born by request of some client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatEvent {
    chat: String,
    client: String,
    time: SystemTime,
    kind: ChatEventKind,
    seq: usize,
}

impl fmt::Display for ChatEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dt: DateTime<Local> = self.time.clone().into();

        match &self.kind {
            ChatEventKind::SentMessage(msg) => write!(
                f,
                "[{}]\t{} {} {}: {}",
                dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                self.client.bold().green(),
                "->".green(),
                self.chat.bold().green(),
                msg.italic()
            ),
            ChatEventKind::Connected() => {
                write!(
                    f,
                    "[{}]\t{} {} {}",
                    dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                    self.client.bold().green(),
                    "connected to".green(),
                    self.chat.bold().green()
                )
            }
            ChatEventKind::Disconnected() => write!(
                f,
                "[{}]\t{} {} {}",
                dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                self.client.bold().green(),
                "disconnected from".green(),
                self.chat.bold().green()
            ),
            ChatEventKind::Created() => write!(
                f,
                "[{}]\t{} {} {}",
                dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                self.client.bold().green(),
                "created".green(),
                self.chat.bold().green()
            ),
        }
    }
}

impl ChatEvent {
    pub fn message_sent(chat: String, client: String, msg: String, seq: usize) -> Self {
        Self {
            chat,
            client,
            time: SystemTime::now(),
            kind: ChatEventKind::SentMessage(msg),
            seq,
        }
    }

    pub fn client_connected(chat: String, client: String, seq: usize) -> Self {
        Self {
            chat,
            client,
            time: SystemTime::now(),
            kind: ChatEventKind::Connected(),
            seq,
        }
    }

    pub fn client_disconnected(chat: String, client: String, seq: usize) -> Self {
        Self {
            chat,
            client,
            time: SystemTime::now(),
            kind: ChatEventKind::Disconnected(),
            seq,
        }
    }

    pub fn chat_created(client: String, chat: String, seq: usize) -> Self {
        Self {
            chat,
            client,
            time: SystemTime::now(),
            kind: ChatEventKind::Created(),
            seq,
        }
    }

    pub fn new_with_kind(chat: String, client: String, kind: ChatEventKind, seq: usize) -> Self {
        Self {
            chat,
            client,
            time: SystemTime::now(),
            kind,
            seq,
        }
    }
}

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
