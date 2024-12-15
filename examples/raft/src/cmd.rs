use dsbuild::Message;
use serde::{Deserialize, Serialize};

pub type KeyType = String;
pub type ValueType = String;

//////////////////////////////////////////////////////////////////////////////////////////

/// Composed of server id and command sequence number
///
/// Every command repsonsible server, which is current leader.
/// In case of server fail, client can not get response on
/// committed command.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
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

const COMMAND: &str = "command";

impl From<Message> for Command {
    fn from(message: Message) -> Self {
        assert_eq!(message.get_tip(), COMMAND);
        message.get_data::<Command>().unwrap()
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
            command_id,
        }
    }
}

const COMMAND_REPLY: &str = "command_reply";

impl From<Message> for Reply {
    fn from(message: Message) -> Self {
        assert_eq!(message.get_tip(), COMMAND_REPLY);
        message.get_data::<Reply>().unwrap()
    }
}

impl From<Reply> for Message {
    fn from(reply: Reply) -> Self {
        Message::new(COMMAND_REPLY, &reply).unwrap()
    }
}
