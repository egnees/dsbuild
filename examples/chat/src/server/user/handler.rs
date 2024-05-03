use serde::{Deserialize, Serialize};
use tokio::sync::MutexGuard;

use crate::{client::requests::ClientRequestKind, server::chat::event::ChatEventKind};

use super::state::UserState;

pub struct RequestHandler<'a> {
    pub user_state: &'a MutexGuard<'a, UserState>,
    pub request: ClientRequestKind,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct RelatedChatEventKind {
    pub kind: ChatEventKind,
    pub chat: String,
}

impl<'a> RequestHandler<'a> {
    pub async fn handle(self) -> Result<RelatedChatEventKind, String> {
        match self.request {
            ClientRequestKind::SendMessage(msg) => match &self.user_state.connected_chat {
                Some(chat) => Ok(RelatedChatEventKind {
                    kind: ChatEventKind::SentMessage(msg),
                    chat: chat.clone(),
                }),
                None => Err("not connected to chat".into()),
            },
            ClientRequestKind::Create(chat) => {
                if self.user_state.connected_chat.is_none() {
                    Ok(RelatedChatEventKind {
                        kind: ChatEventKind::Created(),
                        chat,
                    })
                } else {
                    Err("user connect to chat".into())
                }
            }
            ClientRequestKind::Connect(chat) => {
                if self.user_state.connected_chat.is_none() {
                    Ok(RelatedChatEventKind {
                        kind: ChatEventKind::Connected(),
                        chat,
                    })
                } else {
                    Err("already connected to chat".into())
                }
            }
            ClientRequestKind::Disconnect => match &self.user_state.connected_chat {
                None => Err("not connected to chat".into()),
                Some(chat) => Ok(RelatedChatEventKind {
                    kind: ChatEventKind::Disconnected(),
                    chat: chat.clone(),
                }),
            },
        }
    }
}
