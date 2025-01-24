use dsbuild::Message;
use serde::{Deserialize, Serialize};

use crate::cmd::{CommandId, CommandReply, KeyType, ValueType};

/// Represents request on reading some value in database
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ReadValueRequest {
    pub key: KeyType,

    // use here command id as substitution
    pub request_id: CommandId,

    // minimal commit id node need to answer on request.
    pub min_commit_id: Option<i64>,
}

pub const READ_VALUE_REQUEST: &str = "read_value_request";

impl From<Message> for ReadValueRequest {
    fn from(message: Message) -> Self {
        assert_eq!(message.tip(), READ_VALUE_REQUEST);
        message.data::<ReadValueRequest>().unwrap()
    }
}

impl From<ReadValueRequest> for Message {
    fn from(request: ReadValueRequest) -> Self {
        Message::new(READ_VALUE_REQUEST, &request).unwrap()
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum LocalResponseType {
    Unavailable(),
    ReadValue(Option<ValueType>),
    RedirectedTo(usize, Option<i64>), // min needed commit id
    Command(CommandReply),
}

/// Represents response on reading value request or
/// some command which must be redirected to leader
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct LocalResponse {
    // id of request which is responsed
    pub request_id: CommandId,
    pub tp: LocalResponseType,
}

impl LocalResponse {
    pub fn new(request_id: CommandId, tp: LocalResponseType) -> Self {
        Self { request_id, tp }
    }
}

pub const LOCAL_RESPONSE: &str = "local_response";

impl From<Message> for LocalResponse {
    fn from(message: Message) -> Self {
        assert_eq!(message.tip(), LOCAL_RESPONSE);
        message.data::<LocalResponse>().unwrap()
    }
}

impl From<LocalResponse> for Message {
    fn from(response: LocalResponse) -> Self {
        Message::new(LOCAL_RESPONSE, &response).unwrap()
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Initialization request
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct InitializeRequest {}

pub const INITIALIZE_REQUEST: &str = "initialize_request";

impl From<Message> for InitializeRequest {
    fn from(message: Message) -> Self {
        assert_eq!(message.tip(), INITIALIZE_REQUEST);
        message.data::<InitializeRequest>().unwrap()
    }
}

impl From<InitializeRequest> for Message {
    fn from(request: InitializeRequest) -> Self {
        Message::new(INITIALIZE_REQUEST, &request).unwrap()
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Initialization request
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct InitializeResponse {
    pub seq_num: usize,
}

pub const INITIALIZE_RESPONSE: &str = "initialize_response";

impl From<Message> for InitializeResponse {
    fn from(message: Message) -> Self {
        assert_eq!(message.tip(), INITIALIZE_RESPONSE);
        message.data::<InitializeResponse>().unwrap()
    }
}

impl From<InitializeResponse> for Message {
    fn from(response: InitializeResponse) -> Self {
        Message::new(INITIALIZE_RESPONSE, &response).unwrap()
    }
}
