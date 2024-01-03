//! Isolated process implementation.

use crate::common::{context::Context, message::Message, process::Process};

#[derive(Clone)]
pub struct IsolatedProcess {
    // need timers cnt
    need_cnt: u32,
    // fired timers cnt
    fired_cnt: u32,
    // delay in seconds
    one_timer_delay: f64,
}

impl IsolatedProcess {
    const TIMER_NAME: &'static str = "TIMER";

    pub fn get_fired_cnt(&self) -> u32 {
        self.fired_cnt
    }

    fn set_timer(&self, ctx: &mut dyn Context) {
        ctx.set_timer(Self::TIMER_NAME.to_owned(), self.one_timer_delay);
    }

    pub fn new(need_cnt: u32, one_timer_delay: f64) -> Self {
        Self {
            need_cnt,
            fired_cnt: 0,
            one_timer_delay,
        }
    }
}

impl Process for IsolatedProcess {
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        self.set_timer(ctx);

        Ok(())
    }

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

    fn on_message(
        &mut self,
        _msg: Message,
        _from: String,
        _ctx: &mut dyn Context,
    ) -> Result<(), String> {
        Err("No messages for isolated process".to_owned())
    }
}
