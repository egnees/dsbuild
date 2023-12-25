use crate::common::message::Message;

#[derive(Clone, Debug, PartialEq)]
pub struct Address {
    pub host: String,
    pub port: u16,
    pub process_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessSendRequest {
    pub sender_address: Address,
    pub receiver_address: Address,
    pub message: Message,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessSendResponse {
    pub status: String,
}
