use crate::real_mode::events::Event;

use super::defs::*;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

#[async_trait]
pub trait TimerSetter {
    async fn set_timer(request: SetTimerRequest, sender: Sender<Event>);
}
