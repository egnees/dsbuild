use async_trait::async_trait;

use crate::real_mode::events::Event;

use super::defs::*;
use super::timer_setter::TimerSetter;
use tokio::sync::mpsc::Sender;

use async_io::Timer;

use std::time::Duration;

#[derive(Default)]
pub struct BasicTimerSetter {}

#[async_trait]
impl TimerSetter for BasicTimerSetter {
    async fn set_timer(request: SetTimerRequest, sender: Sender<Event>) {
        Timer::after(Duration::from_secs_f64(request.delay)).await;

        // Ignore result, because send error means system has been shutdown, which is normal behaviour.
        let _ = sender
            .send(Event::TimerFired {
                process_name: request.process,
                timer_name: request.timer_name,
            })
            .await;
    }
}
