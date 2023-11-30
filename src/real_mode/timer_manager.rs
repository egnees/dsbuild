extern crate timer;
extern crate chrono;

use timer::{Guard, Timer};

use super::events::Event;

use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, Mutex};

pub struct TimerManager {
    event_queue: Arc<Mutex<VecDeque<Event>>>,
    pending_timers: Arc<Mutex<HashMap<String, (Timer, Guard)>>>,
}

impl TimerManager {
    pub fn new(event_queue: Arc<Mutex<VecDeque<Event>>>) -> Self {
        TimerManager { 
            event_queue,
            pending_timers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_timer(&mut self, name: &str, delay: f64, overwrite: bool) {
        let timer_name = name.to_string();

        let mut pending_timers_lock = 
                    self.pending_timers.lock().unwrap();

        if pending_timers_lock.contains_key(&timer_name) && !overwrite {
            return;
        } else if pending_timers_lock.contains_key(&timer_name) && overwrite {
            pending_timers_lock.remove(&timer_name);
        }

        let timer = Timer::new();

        let delay_ms = (delay * 1000.0).round() as i64;
        let delay_wrapper = chrono::Duration::milliseconds(delay_ms);

        let timer_name_copy = timer_name.clone();
        let event_queue_copy = self.event_queue.clone();
        let pending_timers_copy = self.pending_timers.clone();

        let guard = 
            timer.schedule_with_delay(delay_wrapper,
            move || {
                    let event = Event::TimerFired { name: timer_name_copy.clone() };
                    event_queue_copy.lock().unwrap().push_back(event);
                    pending_timers_copy.lock().unwrap().remove(&timer_name_copy);
                }
        );

        pending_timers_lock.insert(timer_name, (timer, guard));
    }

    pub fn cancel_timer(&mut self, name: &str) {
        let timer_name = name.to_string();
        self.pending_timers.lock().unwrap().remove(&timer_name);
    }

    pub fn cancel_all_timers(&mut self) {
        self.pending_timers.lock().unwrap().clear();
    }

    pub fn have_timers(&self) -> bool {
        !self.pending_timers.lock().unwrap().is_empty()
    }
}