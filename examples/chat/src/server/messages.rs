use core::fmt;
///! Definition of messages which can be produces by server.
use dsbuild::{Address, Message};
use serde::{Deserialize, Serialize};

use super::event::ChatEvent;

/// Represents response from server to client request.
pub type RequestResponse = Result<(), String>;

/// Represents messages from server to the client.
///
/// Server can send even ack/nack of client requests,
/// or events which were appeared in the client's chat.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ServerMessage {
    RequestResponse(u64, RequestResponse), // Response on request (request_id, response).
    ChatEvent(String, ChatEvent),          // Name of the chat and chat events.
}

impl fmt::Display for ServerMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerMessage::RequestResponse(id, response) => {
                write!(f, "[id={}] response={:?}", id, response)
            }
            ServerMessage::ChatEvent(chat, event) => {
                write!(f, "chat={}, event={}", chat, event.to_string())
            }
        }
    }
}

impl From<ServerMessage> for Message {
    fn from(value: ServerMessage) -> Self {
        Message::borrow_new("SERVER_MESSAGE", value).unwrap()
    }
}

pub struct DirectedServerMessage {
    pub msg: ServerMessage,
    pub to: Address,
}
