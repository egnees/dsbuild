//! Implementation of Pong Process (Ponger).

use crate::common::{context::Context, message::Message, process::Process};

// PongProcess waits for consecutive ping messages,
// and answers consecutive pongs messages with specified delay.
#[derive(Clone)]
pub struct PongProcess {
    // window of inactivity after that pong process will be stopped
    delay: f64,
}

impl PongProcess {
    const TIMER_NAME: &'static str = "PONG_TIMER";
    pub const PONG_TIP: &'static str = "PONG";
    pub const PING_TIP: &'static str = "PING";

    fn set_timer(&self, ctx: &mut dyn Context) {
        ctx.set_timer(Self::TIMER_NAME.to_owned(), self.delay);
    }

    pub fn new(delay: f64) -> Self {
        Self { delay }
    }
}

impl Process for PongProcess {
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        self.set_timer(ctx);
        Ok(())
    }

    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
        assert_eq!(name, Self::TIMER_NAME);
        ctx.stop_process();
        Ok(())
    }

    fn on_message(
        &mut self,
        msg: Message,
        from: String,
        ctx: &mut dyn Context,
    ) -> Result<(), String> {
        assert_eq!(msg.get_tip(), Self::PING_TIP);

        let requested_pong = msg.get_data::<u32>()?;
        let answer = Message::borrow_new(Self::PONG_TIP, requested_pong)?;
        ctx.send_message(answer, from);
        self.set_timer(ctx);
        Ok(())
    }
}
