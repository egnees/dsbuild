//! Definitions for working with storage.

use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use tokio::sync::Mutex;

/// Represents type for file lock.
/// Can be used to get [`FileGuard`].
pub type FileLock = Arc<Mutex<()>>;

/// Represents thread-unsafe file manager.
pub struct FileManager {
    locks: HashMap<String, FileLock>,
    mount_dir: String,
}

impl FileManager {
    /// Create a new file manager
    pub fn new(mount_dir: String) -> Self {
        Self {
            locks: HashMap::new(),
            mount_dir,
        }
    }

    /// Returns mount directory
    pub fn get_mount_dir(&self) -> &str {
        self.mount_dir.as_str()
    }

    /// Register file with specified name.
    /// Returns [`Some`] if file was not present.
    pub fn register_file(&mut self, name: String) -> Option<FileLock> {
        if let Entry::Vacant(e) = self.locks.entry(name) {
            let lock = Arc::new(Mutex::new(()));
            e.insert(lock.clone());
            Some(lock)
        } else {
            None
        }
    }

    /// Returns guard on file, which guarantees file will be locked until guard wont be dropped.
    /// In such file no presents, returns error.
    pub fn get_file_lock(&mut self, name: &str) -> Option<FileLock> {
        self.locks.get(name).cloned()
    }
}
