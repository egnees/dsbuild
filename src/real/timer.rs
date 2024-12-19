//! Definition of time management objects.

use std::collections::HashMap;

use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

/// Responsible for settings and cancelling timers.
/// Not thread-safe.
pub(crate) struct TimerManager {
    pending_timers: HashMap<String, JoinHandle<()>>,
    sender: Sender<String>,
}

impl TimerManager {
    // Create new timer manager.
    pub fn new(sender: Sender<String>) -> Self {
        Self {
            pending_timers: HashMap::default(),
            sender,
        }
    }

    /// Set timer with specified name, delay and overwrite strategy.
    /// When timer fires, it is name will be passed to sender.
    pub fn set_timer(&mut self, name: String, delay: f64, overwrite: bool) {
        if !overwrite && self.pending_timers.contains_key(&name) {
            return;
        }
        if overwrite {
            self.cancel_timer(&name);
        }
        let name_clone = name.clone();
        let sender = self.sender.clone();
        let handler = tokio::spawn(async move {
            sleep(Duration::from_secs_f64(delay)).await;
            sender.send(name).await.unwrap();
        });
        self.pending_timers.insert(name_clone, handler);
    }

    /// Cancel timer with specified name.
    pub fn cancel_timer(&mut self, name: &str) {
        if let Some(timer) = self.pending_timers.remove(name) {
            timer.abort();
        }
    }

    #[allow(dead_code)]
    /// Cancel all pending timers.
    pub fn cancel_all_timers(&mut self) {
        for task in self.pending_timers.values_mut() {
            task.abort();
        }
        self.pending_timers.clear();
    }
}
