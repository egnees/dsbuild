//! Definition of the chat manager.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Mutex, MutexGuard};

use super::chat::Chat;

/// Can be used to lock `[ChatHistoryManager]`.
pub type ChatLocker = Arc<Mutex<Chat>>;

/// Can be used to work with chat exclusive.
pub type ChatGuard<'a> = MutexGuard<'a, Chat>;

/// Responsible for providing chat locks.
/// In general, there should be only one chats manager.
/// It should provide persistence for the chats history,
/// but information about users connections can be reset on shutdown.
#[derive(Default, Clone)]
pub struct ChatsManager {
    chats: HashMap<String, ChatLocker>,
}

impl ChatsManager {
    /// Returns chat locker.
    /// If no locker presents, creates a new one.
    pub fn get_chat_lock(&mut self, chat: &str) -> ChatLocker {
        self.chats
            .entry(chat.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(Chat::new(chat.to_owned()))))
            .clone()
    }
}
