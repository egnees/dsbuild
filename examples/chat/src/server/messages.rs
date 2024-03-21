///! Definition of messages which can be produces by server.
use std::time::SystemTime;

use dsbuild::Message;
use serde::{Deserialize, Serialize};

/// Represents response from server to client request.
pub type RequestResponse = Result<(), String>;

/// Represents event in the chat.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChatEventKind {
    SentMessage(String), // Client sent message,
    Connected(),         // Client connected to server,
    Disconnected(),      // Client disconnected from server,
    Created(),           // Client created chat.
}

/// Represents chat event born by request of some client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatEvent {
    chat: String,
    client: String,
    time: SystemTime,
    kind: ChatEventKind,
}

/// Represents messages from server to the client.
///
/// Server can send even ack/nack of client requests,
/// or events which were appeared in the client's chat.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessageKind {
    RequestResponse(usize, RequestResponse), // Response on request.
    ChatEvents(String, Vec<ChatEvent>),      // Name of the chat and chat events.
}

/// Represents messages from server to the client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMessage {
    pub server: String,
    pub time: SystemTime,
    pub kind: ServerMessageKind,
}

impl From<ServerMessage> for Message {
    fn from(value: ServerMessage) -> Self {
        Message::borrow_new("SERVER_MESSAGE", value).unwrap()
    }
}
