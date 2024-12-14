use serde::{Deserialize, Serialize};

pub type KeyType = String;
pub type ValueType = String;

/// Represents type of entry log
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum LogEntryType {
    Create(KeyType),
    Update(KeyType, ValueType),
    Read(KeyType),
    Delete(KeyType),
    CAS(KeyType, ValueType, ValueType), // compare (with $2) and swap (with $3)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct LogEntry {
    pub term: usize,
    pub tp: LogEntryType,
}

/// Represents array of log entries
pub type LogEntries = Vec<LogEntry>;
