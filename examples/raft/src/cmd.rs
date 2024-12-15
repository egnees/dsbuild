use serde::{Deserialize, Serialize};

pub type KeyType = String;
pub type ValueType = String;

/// Allows to get async reply
pub type RequestToken = usize;

//////////////////////////////////////////////////////////////////////////////////////////

/// Composed of server id and command seq number
pub type CommandId = (usize, usize);

/// Represents commands
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum CommandType {
    Create(KeyType),
    // Read?
    Update(KeyType, ValueType),
    Delete(KeyType),
    Cas(KeyType, ValueType, ValueType), // compare (with $2) and swap (with $3)
}

/// Represents command requested by user
///
/// Every command will be responsed as corresponding log will commited by leader.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Command {
    pub tp: CommandType,
    pub id: CommandId,
}

impl Command {
    pub fn new(tp: CommandType, id: CommandId) -> Self {
        Self {
            tp, 
            id
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Represents reply on command
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Reply {
    /// Corresponds to http status codes
    pub status: u16,

    /// Status info
    pub info: String,

    /// Id of command replying to
    pub command_id: CommandId,
}

impl Reply {
    pub fn new(status: u16, info: &str, command_id: CommandId) -> Self {
        Self {
            status,
            info: info.to_owned(),
            command_id
        }
    }
}
