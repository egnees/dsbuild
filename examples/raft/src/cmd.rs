use dsbuild::Message;
use serde::{Deserialize, Serialize};

pub type KeyType = String;
pub type KeyTypeRef = str;

pub type ValueType = String;
pub type ValueTypeRef = str;

//////////////////////////////////////////////////////////////////////////////////////////

/// Composed of server id and command sequence number
///
/// Every command repsonsible server, which is current leader.
/// In case of server fail, client can not get response on
/// committed command.
///
/// (responsible_server, sequence_number)
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub struct CommandId(pub usize, pub usize);

impl CommandId {
    pub fn responsible_server(&self) -> usize {
        self.0
    }

    pub fn sequence_number(&self) -> usize {
        self.1
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Represents commands
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum CommandType {
    Create(KeyType),
    // Read?
    Update(KeyType, ValueType),
    Delete(KeyType),
    Cas(KeyType, ValueType, ValueType), // compare (with $2) and swap (with $3)
}

impl CommandType {
    pub fn create(key: &KeyTypeRef) -> Self {
        Self::Create(key.to_owned())
    }

    pub fn update(key: &KeyTypeRef, value: &ValueTypeRef) -> Self {
        Self::Update(key.to_owned(), value.to_owned())
    }

    pub fn delete(key: &KeyTypeRef) -> Self {
        Self::Delete(key.to_owned())
    }

    pub fn cas(key: &KeyTypeRef, compare: &ValueTypeRef, swap: &ValueTypeRef) -> Self {
        Self::Cas(key.to_owned(), compare.to_owned(), swap.to_owned())
    }
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
        Self { tp, id }
    }
}

pub const COMMAND: &str = "command";

impl From<Message> for Command {
    fn from(message: Message) -> Self {
        assert_eq!(message.tip(), COMMAND);
        message.data::<Command>().unwrap()
    }
}

impl From<Command> for Message {
    fn from(command: Command) -> Self {
        Message::new(COMMAND, &command).unwrap()
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Represents reply on command
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CommandReply {
    /// Corresponds to http status codes
    pub status: u16,

    /// Status info
    pub info: String,

    /// Id of command replying to
    pub command_id: CommandId,
}

impl CommandReply {
    pub fn new(status: u16, info: &str, command_id: CommandId) -> Self {
        Self {
            status,
            info: info.to_owned(),
            command_id,
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

pub const CREATED_CODE: u16 = 201;
pub const ALREADY_EXISTS_CODE: u16 = 409;
pub const UPDATED_CODE: u16 = 202;
pub const NOT_FOUND_CODE: u16 = 404;

/// Accepted but not updated (on cas)
pub const NOT_UPDATED_CODE: u16 = 202;

pub const DELETED_CODE: u16 = 204;
