use serde::{Deserialize, Serialize};

use crate::cmd::Command;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct LogEntry {
    pub term: usize,
    pub command: Command,
}

impl LogEntry {
    pub fn new(term: usize, command: Command) -> Self {
        Self { term, command }
    }
}

/// Represents array of log entries
pub type LogEntries = Vec<LogEntry>;
