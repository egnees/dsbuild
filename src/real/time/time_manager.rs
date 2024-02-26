//! Definition of [`time_manager`][`TimeManager`], which is responsible for managing time and timers.

use std::{collections::HashMap, marker::PhantomData};

use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::real::events::Event;

use super::{defs::SetTimerRequest, timer_setter::TimerSetter};

/// Specifies id of timer.
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
struct TimerID {
    process_name: String,
    timer_name: String,
}

/// Specifies [`time_manager`][`TimeManager`], 
/// which can use some [`TimerSetter`] implementation to set and manage timers.
#[derive(Default)]
pub struct TimeManager<T: TimerSetter> {
    pending_timers: HashMap<TimerID, JoinHandle<()>>,
    _phantom: PhantomData<T>,
}

/// Implementation [`TimeManager`].
impl<T: TimerSetter> TimeManager<T> {
    /// Create new time manager.
    pub fn new() -> TimeManager<T> {
        Self {
            pending_timers: HashMap::default(),
            _phantom: PhantomData,
        }
    }

    /// Allows to set the timer with specified.
    /// * `sender` - Sender end of channel, in which [`timer fired event`][Event::TimerFired] will be sent after triggering.
    /// * `process_name` - name of process, which created the timer.
    /// * `timer_name` - name of timer.
    /// * `overwrite` - specifies whether to overwrite existing timer or not.
    pub fn set_timer(
        &mut self,
        sender: Sender<Event>,
        process_name: &str,
        timer_name: &str,
        delay: f64,
        overwrite: bool,
    ) {
        let timer_id = TimerID {
            process_name: process_name.into(),
            timer_name: timer_name.into(),
        };

        if self.pending_timers.contains_key(&timer_id) && !overwrite {
            return;
        }

        let request = SetTimerRequest {
            process: process_name.into(),
            timer_name: timer_name.into(),
            delay,
        };

        if self.pending_timers.contains_key(&timer_id) {
            self.pending_timers
                .get_mut(&timer_id)
                .expect("Incorrect implementation")
                .abort();
        }

        let timer = tokio::spawn(T::set_timer(request, sender));

        self.pending_timers.insert(timer_id, timer);
    }

    pub fn cancel_timer(&mut self, process_name: &str, timer_name: &str) {
        let timer_id = TimerID {
            process_name: process_name.into(),
            timer_name: timer_name.into(),
        };

        if !self.pending_timers.contains_key(&timer_id) {
            return;
        }

        let handler = self
            .pending_timers
            .remove(&timer_id)
            .expect("Incorrect implementation. Probably data race appeared.");
        handler.abort();
    }

    pub fn cancel_all_timers(&mut self) {
        for handler in self.pending_timers.values_mut() {
            handler.abort();
        }

        self.pending_timers.clear();
    }
}
