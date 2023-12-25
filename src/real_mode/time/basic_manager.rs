use async_trait::async_trait;

use super::manager::TimeManager;
use super::defs::*;
use tokio::sync::mpsc::Sender;

use async_io::Timer;

use std::time::Duration;

pub struct BasicTimeManager {}

#[async_trait]
impl TimeManager for BasicTimeManager {
    async fn set_timer(request: SetTimerRequest, sender: Sender<TimerFiredEvent>) {
        Timer::after(Duration::from_secs_f64(request.delay)).await;
        let _ = sender.send(TimerFiredEvent { process: request.process.clone(), timer_name: request.timer_name.clone() }).await;
    }
}