///! Definition of requests from client to server which can appear in the system.
use std::time::SystemTime;

use dsbuild::Message;
use serde::{Deserialize, Serialize};

/// Represents request from client to server.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ClientRequest {
    pub id: usize,
    pub client: String,
    pub password: String,
    pub time: SystemTime,
    pub kind: ClientRequestKind,
}

/// Represents types of the client request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ClientRequestKind {
    Auth,                // Register client with specified name.
    SendMessage(String), // Send message with specified content in the currently connected chat.
    Create(String),      // Create chat with specified name.
    Connect(String),     // Connect to chat with specified name.
    Disconnect,          // Disconnect from chat with specified name.
}

/// Allows to create [`Message`] from [`ClientRequest`].
impl From<ClientRequest> for Message {
    fn from(value: ClientRequest) -> Self {
        Message::borrow_new("CLIENT_REQUEST", value).unwrap()
    }
}

/// Allows to create [`Message`] from [`ClientRequestKind`].
impl From<ClientRequestKind> for Message {
    fn from(value: ClientRequestKind) -> Self {
        Message::borrow_new("CLIENT_REQUEST_KIND", value).unwrap()
    }
}
