use dsbuild::Message;
use serde::{Deserialize, Serialize};

use crate::cmd::{CommandId, KeyType, ValueType};

/// Represents request on reading some value in database
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ReadValueRequest {
    pub key: KeyType,

    // use here command id as substitution
    pub request_id: CommandId,

    // minimal commit id node need to answer on request.
    pub min_commit_id: Option<usize>,
}

pub const READ_VALUE_REQUEST: &str = "read_value_request";

impl From<Message> for ReadValueRequest {
    fn from(message: Message) -> Self {
        assert_eq!(message.get_tip(), READ_VALUE_REQUEST);
        message.get_data::<ReadValueRequest>().unwrap()
    }
}

impl From<ReadValueRequest> for Message {
    fn from(request: ReadValueRequest) -> Self {
        Message::new(READ_VALUE_REQUEST, &request).unwrap()
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Represents response on reading value
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ReadValueResponse {
    pub value: Option<ValueType>,

    // id of request which is responsed
    pub request_id: CommandId,

    /// if node can not answer on request,
    /// it redirects client to the leader
    pub redirected_to: Option<usize>,
}

pub const READ_VALUE_RESPONSE: &str = "read_value_response";

impl From<Message> for ReadValueResponse {
    fn from(message: Message) -> Self {
        assert_eq!(message.get_tip(), READ_VALUE_RESPONSE);
        message.get_data::<ReadValueResponse>().unwrap()
    }
}

impl From<ReadValueResponse> for Message {
    fn from(response: ReadValueResponse) -> Self {
        Message::new(READ_VALUE_RESPONSE, &response).unwrap()
    }
}
