use std::{collections::HashMap, marker::PhantomData, sync::Mutex};

use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::real_mode::events::Event;

use super::{
    basic_timer_setter::BasicTimerSetter, defs::SetTimerRequest, timer_setter::TimerSetter,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
struct TimerID {
    process_name: String,
    timer_name: String,
}

#[derive(Default)]
struct TimeManager<T: TimerSetter> {
    pending_timers: HashMap<TimerID, JoinHandle<()>>,
    _phantom: PhantomData<T>,
}

impl<T: TimerSetter> TimeManager<T> {
    pub fn new() -> TimeManager<T> {
        Self {
            pending_timers: HashMap::default(),
            _phantom: PhantomData,
        }
    }

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

static MANAGER: Mutex<Option<TimeManager<BasicTimerSetter>>> = Mutex::new(None);

pub fn init() {
    let mut guard = MANAGER.lock().unwrap();
    *guard = Some(TimeManager::new());
}

pub fn set_timer(
    sender: Sender<Event>,
    process_name: &str,
    timer_name: &str,
    delay: f64,
    overwrite: bool,
) {
    let mut guard = MANAGER.lock().expect("Can not get timer manager guard");
    let timer_manager_ref = guard.as_mut().expect("Timer manager not initiated");

    timer_manager_ref.set_timer(sender, process_name, timer_name, delay, overwrite);
}

pub fn cancel_timer(process_name: &str, timer_name: &str) {
    let mut guard = MANAGER.lock().expect("Can not get timer manager guard");
    let timer_manager_ref = guard.as_mut().expect("Timer manager not initiated");

    timer_manager_ref.cancel_timer(process_name, timer_name);
}

pub fn cancel_all_timers() {
    let mut guard = MANAGER.lock().expect("Can not get timer manager guard");
    let timer_manager_ref = guard.as_mut().expect("Timer manager not initiated");

    timer_manager_ref.cancel_all_timers();
}
