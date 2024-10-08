//! Definition of requests from client to server which can appear in the system.
use std::fmt;
use std::time::Duration;
use std::time::SystemTime;

use chrono::DateTime;
use chrono::Local;
use colored::Colorize;
use dsbuild::Address;
use dsbuild::Message;
use serde::{Deserialize, Serialize};

/// Represents request from client to server.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ClientRequest {
    pub id: u64,
    pub client: String,
    pub password: String,
    pub time: Option<f64>,
    pub kind: ClientRequestKind,
    pub addr: Option<Address>,
}

impl Eq for ClientRequest {}

impl fmt::Display for ClientRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dt: DateTime<Local> = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs_f64(self.time.unwrap_or(0.0)))
            .unwrap()
            .into();
        write!(
            f,
            "[{}]\t [{}{}] {}: {}",
            dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
            "id=".italic(),
            self.id,
            self.client.bold().green().underline(),
            self.kind
        )
    }
}

/// Represents types of the client request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ClientRequestKind {
    SendMessage(String), // Send message with specified content in the currently connected chat.
    Create(String),      // Create chat with specified name.
    Connect(String),     // Connect to chat with specified name.
    Disconnect,          // Disconnect from chat with specified name.
    Status,              // Status of the client.
}

impl fmt::Display for ClientRequestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientRequestKind::SendMessage(msg) => {
                write!(f, "{}", msg.italic())
            }
            ClientRequestKind::Create(chat) => {
                write!(
                    f,
                    "{} {}",
                    "create".italic(),
                    chat.italic().underline().bold().green()
                )
            }
            ClientRequestKind::Connect(chat) => {
                write!(
                    f,
                    "{} {}",
                    "connect".italic(),
                    chat.italic().underline().bold().green()
                )
            }
            ClientRequestKind::Disconnect => {
                write!(f, "{}", "disconnect".italic())
            }
            ClientRequestKind::Status => write!(f, "{}", "status".italic()),
        }
    }
}

/// Allows to create [`Message`] from [`ClientRequest`].
impl From<ClientRequest> for Message {
    fn from(value: ClientRequest) -> Self {
        Message::new("client_request", &value).unwrap()
    }
}

/// Allows to create [`Message`] from [`ClientRequestKind`].
impl From<ClientRequestKind> for Message {
    fn from(value: ClientRequestKind) -> Self {
        Message::new("client_request_kind", &value).unwrap()
    }
}

/// Allows to build requests comfortable.
#[derive(Debug, Clone)]
pub struct RequestBuilder {
    id: u64,
    client: String,
    password: String,
}

impl RequestBuilder {
    pub fn new(client: String, password: String) -> Self {
        Self {
            id: 0,
            client,
            password,
        }
    }

    pub fn send_message_request(&mut self, message: String) -> ClientRequest {
        self.build_with_kind(ClientRequestKind::SendMessage(message))
    }

    pub fn create_request(&mut self, chat_name: String) -> ClientRequest {
        self.build_with_kind(ClientRequestKind::Create(chat_name))
    }

    pub fn connect_request(&mut self, chat_name: String) -> ClientRequest {
        self.build_with_kind(ClientRequestKind::Connect(chat_name))
    }

    pub fn disconnect_request(&mut self) -> ClientRequest {
        self.build_with_kind(ClientRequestKind::Disconnect)
    }

    fn next_id(&mut self) -> u64 {
        self.id += 1;
        self.id
    }

    pub fn build_with_kind(&mut self, kind: ClientRequestKind) -> ClientRequest {
        ClientRequest {
            id: self.next_id(),
            client: self.client.clone(),
            password: self.password.clone(),
            time: None,
            kind,
            addr: None,
        }
    }
}
