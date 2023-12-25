use super::defs::*;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

#[async_trait]
pub trait AsyncMessenger {
    async fn send(request: ProcessSendRequest) -> Result<ProcessSendResponse, String>;

    async fn listen(
        host: &str,
        port: u16,
        pass_to: Sender<ProcessSendRequest>,
    ) -> Result<(), String>;
}