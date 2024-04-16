//! Definitions for working with storage.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Mutex, MutexGuard};

/// Represents type for file lock.
/// Can be used to get [`FileGuard`].
pub type FileLock = Arc<Mutex<()>>;

/// Represents type for guard on file.
/// Owning it guarantees exclusive access to file.
pub type FileGuard<'a> = MutexGuard<'a, ()>;

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
        if self.locks.contains_key(&name) {
            None
        } else {
            let lock = Arc::new(Mutex::new(()));
            self.locks.insert(name, lock.clone());
            Some(lock)
        }
    }

    /// Returns guard on file, which guarantees file will be locked until guard wont be dropped.
    /// In such file no presents, returns error.
    pub fn get_file_lock(&mut self, name: &str) -> Option<FileLock> {
        self.locks.get(name).map(|lock| lock.clone())
    }
}
