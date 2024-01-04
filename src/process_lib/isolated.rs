//! Implementation of [`IsolatedProcess`].

use crate::common::{context::Context, message::Message, process::Process};

/// Isolated process is not connected to the network.
/// It just sets consecutive timers one by one,
/// while the last timer won't expire.
#[derive(Clone)]
pub struct IsolatedProcess {
    /// Number of timers which must fire before the process will be stopped.
    need_cnt: u32,
    /// Count of already fired timers.
    fired_cnt: u32,
    /// One timer delay.
    one_timer_delay: f64,
}

impl IsolatedProcess {
    /// Name of the timer.
    const TIMER_NAME: &'static str = "TIMER";

    /// Returns the number of fired timers count.
    pub fn get_fired_cnt(&self) -> u32 {
        self.fired_cnt
    }

    /// Assistant function which sets timer with specified delay and name.
    fn set_timer(&self, ctx: &mut dyn Context) {
        ctx.set_timer(Self::TIMER_NAME.to_owned(), self.one_timer_delay);
    }

    /// Creates new isolated process
    /// with specified number of timers to fire and delay of one timer.
    pub fn new(need_cnt: u32, one_timer_delay: f64) -> Self {
        Self {
            need_cnt,
            fired_cnt: 0,
            one_timer_delay,
        }
    }
}

impl Process for IsolatedProcess {
    /// Called when system is started. 
    /// Sets the first timer.
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        self.set_timer(ctx);

        Ok(())
    }

    /// Called when timer is fired.
    /// If after that number of already fired timers is equal
    /// to the total number of timers to fire,
    /// process is stopped.
    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
        assert_eq!(name, Self::TIMER_NAME);

        self.fired_cnt += 1;
        if self.fired_cnt < self.need_cnt {
            self.set_timer(ctx);
        } else {
            ctx.stop_process();
        }

        Ok(())
    }

    /// Isolated process doesn't send any messages and should not receive any messages.
    fn on_message(
        &mut self,
        _msg: Message,
        _from: String,
        _ctx: &mut dyn Context,
    ) -> Result<(), String> {
        Err("No messages for isolated process".to_owned())
    }
}
