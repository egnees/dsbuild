use crate::real_mode::events::Event;

use super::defs::*;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

#[async_trait]
pub trait AsyncMessenger {
    async fn send(request: ProcessSendRequest) -> Result<ProcessSendResponse, String>;

    async fn listen(host: String, port: u16, pass_to: Sender<Event>) -> Result<(), String>;
}
