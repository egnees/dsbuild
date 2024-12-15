use serde::{Deserialize, Serialize};

use crate::cmd::{Command};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct LogEntry {
    pub term: usize,
    pub command: Command,
}

/// Represents array of log entries
pub type LogEntries = Vec<LogEntry>;
