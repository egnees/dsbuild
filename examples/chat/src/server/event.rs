//! Definition of events in chat.

use std::fmt::{Debug, Display, Formatter, Result};
use std::time::Duration;
use std::{cmp::Ordering, time::SystemTime};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use colored::Colorize;

/// Represents event in the chat.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ChatEventKind {
    SentMessage(String), // Client sent message,
    Connected(),         // Client connected to chat,
    Disconnected(),      // Client disconnected from chat,
    Created(),           // Client created chat.
}

/// Represents chat event born by request of some client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatEvent {
    pub chat: String,
    pub user: String,
    pub time: f64,
    pub kind: ChatEventKind,
    pub seq: u64,
}

impl PartialEq for ChatEvent {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
            && self.chat == other.chat
            && self.user == other.user
            && self.kind == other.kind
            && self.time == other.time
    }
}

impl Eq for ChatEvent {}

/// It makes sense to compare two chat events only in context of common chat.
impl Ord for ChatEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.seq.cmp(&other.seq)
    }
}

impl PartialOrd for ChatEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl ChatEvent {
    pub fn message_sent(chat: String, client: String, msg: String, seq: u64) -> Self {
        Self {
            chat,
            user: client,
            time: 0.0,
            kind: ChatEventKind::SentMessage(msg),
            seq,
        }
    }

    pub fn client_connected(chat: String, client: String, seq: u64) -> Self {
        Self {
            chat,
            user: client,
            time: 0.0,
            kind: ChatEventKind::Connected(),
            seq,
        }
    }

    pub fn client_disconnected(chat: String, client: String, seq: u64) -> Self {
        Self {
            chat,
            user: client,
            time: 0.0,
            kind: ChatEventKind::Disconnected(),
            seq,
        }
    }

    pub fn chat_created(client: String, chat: String, seq: u64) -> Self {
        Self {
            chat,
            user: client,
            time: 0.0,
            kind: ChatEventKind::Created(),
            seq,
        }
    }

    pub fn new_with_kind(chat: String, client: String, kind: ChatEventKind, seq: u64) -> Self {
        Self {
            chat,
            user: client,
            time: 0.0,
            kind,
            seq,
        }
    }
}

impl Display for ChatEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let dt: DateTime<Local> = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs_f64(self.time))
            .unwrap()
            .into();

        match &self.kind {
            ChatEventKind::SentMessage(msg) => write!(
                f,
                "[{}]\t{} {} {}: {}",
                dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                self.user.bold().green(),
                "->".green(),
                self.chat.bold().green(),
                msg.italic()
            ),
            ChatEventKind::Connected() => {
                write!(
                    f,
                    "[{}]\t{} {} {}",
                    dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                    self.user.bold().green(),
                    "connected to".green(),
                    self.chat.bold().green()
                )
            }
            ChatEventKind::Disconnected() => write!(
                f,
                "[{}]\t{} {} {}",
                dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                self.user.bold().green(),
                "disconnected from".green(),
                self.chat.bold().green()
            ),
            ChatEventKind::Created() => write!(
                f,
                "[{}]\t{} {} {}",
                dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                self.user.bold().green(),
                "created".green(),
                self.chat.bold().green()
            ),
        }
    }
}
