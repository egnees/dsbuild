use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

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

#[async_trait]
pub trait AsyncMessenger {
    async fn send(request: ProcessSendRequest) -> Result<ProcessSendResponse, String>;

    async fn listen(
        host: &str,
        port: u16,
        pass_to: Sender<ProcessSendRequest>,
    ) -> Result<(), String>;
}
