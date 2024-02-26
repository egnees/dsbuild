//! Definition of [`basic timer setter`][`BasicTimerSetter`] which is responsible for settings timers.

use async_trait::async_trait;

use crate::real::events::Event;

use super::defs::*;
use super::timer_setter::TimerSetter;
use tokio::sync::mpsc::Sender;

use async_io::Timer;

use std::time::Duration;

/// Specifies [`basic timer setter`][`BasicTimerSetter`].
#[derive(Default)]
pub struct BasicTimerSetter {}

/// Implementation [`TimerSetter`] trait for the [`BasicTimerSetter`].
#[async_trait]
impl TimerSetter for BasicTimerSetter {
    /// Allows to set the timer.
    async fn set_timer(request: SetTimerRequest, sender: Sender<Event>) {
        Timer::after(Duration::from_secs_f64(request.delay)).await;

        // Ignore result, because send error means system has been shutdown, which is normal behavior.
        let _ = sender
            .send(Event::TimerFired {
                process_name: request.process,
                timer_name: request.timer_name,
            })
            .await;
    }
}
