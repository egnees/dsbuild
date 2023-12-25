use super::defs::*;

use tokio::sync::mpsc::Sender;
use async_trait::async_trait;


#[async_trait]
pub trait TimeManager {
    async fn set_timer(request: SetTimerRequest, sender: Sender<TimerFiredEvent>);
}