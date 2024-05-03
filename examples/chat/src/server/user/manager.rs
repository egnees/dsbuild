//! Definition of users manager.

use dsbuild::Address;
use tokio::sync::Mutex;

use super::state::UserState;
use std::{collections::HashMap, sync::Arc};

pub type UserLock = Arc<Mutex<UserState>>;

/// Responsible for user locks.
#[derive(Default, Clone)]
pub struct UsersManager {
    locks: HashMap<String, UserLock>,
}

impl UsersManager {
    /// Acquire user lock.
    pub fn get_user_lock(&mut self, name: &str, addr: &Address) -> UserLock {
        self.locks
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(UserState::new(name.to_string(), addr.clone()))))
            .clone()
    }

    /// Get user lock without creating.
    pub fn get_user_lock_without_creating(&self, name: &str) -> Option<UserLock> {
        self.locks.get(name).cloned()
    }
}
