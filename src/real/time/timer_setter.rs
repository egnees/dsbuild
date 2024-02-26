//! Definition of the [`TimerSetter`] trait, which is used by [`super::time_manager::TimeManager`] to set timers.

use crate::real::events::Event;

use super::defs::*;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

/// Specifies [`TimerSetter`] trait, which is used by [`super::time_manager::TimeManager]` to set timers.
#[async_trait]
pub trait TimerSetter {
    async fn set_timer(request: SetTimerRequest, sender: Sender<Event>);
}
