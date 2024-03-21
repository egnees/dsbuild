//! Definitions of methods for [`Client`] input and output interactions.
use dsbuild::{IOProcessWrapper, Message};
use serde::{Deserialize, Serialize};

use crate::server::messages::ChatEvent;

use super::{parser::parse_request, process::Client, requests::ClientRequestKind};

#[derive(Debug, Serialize, Deserialize)]
pub struct InnerInfo {
    pub info: String,
}

impl InnerInfo {
    pub fn new(info: String) -> Self {
        Self { info }
    }
}

/// Represents info which user can get from the process.
#[derive(Debug, Serialize, Deserialize)]
pub enum Info {
    /// Inner info about system state.
    InnerInfo(InnerInfo),
    /// Event in chat.
    ChatEvent(ChatEvent),
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

    // Auth.
    sender.send(ClientRequestKind::Auth.into()).await.unwrap();

    let stdio = async_std::io::stdin();

    loop {
        let mut cmd = String::new();
        tokio::select! {
            Ok(_) = stdio.read_line(&mut cmd) => {
                let request_kind_result = parse_request(&cmd);
                match request_kind_result {
                    Ok(request_kind) => sender.send(request_kind.into()).await.unwrap(),
                    Err(parse_error) => println!("{:?}", parse_error)
                }
            },
            Some(msg) = receiver.recv() => {
                println!("{:?}", msg.get_data::<Info>().unwrap());
            },
            else => break
        }
    }
}
