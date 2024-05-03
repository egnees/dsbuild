//! Definitions of methods for [`Client`] input and output interactions.
use std::fmt;
use std::time::SystemTime;

use dsbuild::{IOProcessWrapper, Message};
use serde::{Deserialize, Serialize};

use colored::Colorize;

use chrono::{DateTime, Local};

use crate::server::event::ChatEvent;

use super::{client::Client, parser::parse_request};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InnerInfo {
    pub info: String,
}

impl InnerInfo {
    pub fn new(info: String) -> Self {
        Self { info }
    }
}

impl fmt::Display for InnerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dt: DateTime<Local> = SystemTime::now().into();
        write!(
            f,
            "[{}]\t{}: {}",
            dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
            "INFO".blue().bold(),
            self.info.italic()
        )
    }
}

/// Represents info which user can get from the process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Info {
    /// Represents inner error information.
    InnerInfo(InnerInfo),
    /// Event in chat.
    ChatEvent(ChatEvent),
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Info::InnerInfo(info) => write!(f, "{}", info),
            Info::ChatEvent(event) => write!(f, "{}", event),
        }
    }
}

impl From<&str> for Info {
    fn from(value: &str) -> Self {
        Info::InnerInfo(InnerInfo::new(value.to_owned()))
    }
}

impl From<ChatEvent> for Info {
    fn from(value: ChatEvent) -> Self {
        Info::ChatEvent(value)
    }
}

impl From<Info> for Message {
    fn from(value: Info) -> Self {
        Message::borrow_new("INFO", value).unwrap()
    }
}

impl From<Message> for Info {
    fn from(value: Message) -> Self {
        value.get_data::<Info>().unwrap()
    }
}
/// Start client io-activity.
pub async fn start_io(wrapper: IOProcessWrapper<Client>) {
    let sender = wrapper.sender;
    let mut receiver = wrapper.receiver;

    let stdio = async_std::io::stdin();

    loop {
        let mut cmd = String::new();
        tokio::select! {
            Ok(_) = stdio.read_line(&mut cmd) => {
                let request_kind_result = parse_request(&cmd);
                match request_kind_result {
                    Ok(request_kind) => sender.send(request_kind.into()).await.unwrap(),
                    Err(parse_error) => println!("{}", parse_error)
                }
            },
            Some(msg) = receiver.recv() => {
                println!("{}", msg.get_data::<Info>().unwrap());
            },
            else => break
        }
    }
}
