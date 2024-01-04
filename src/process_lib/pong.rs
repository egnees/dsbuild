//! Implementation of [`PongProcess`].

use crate::common::{context::Context, message::Message, process::Process};

/// [`PongProcess`] waits for messages with tip equals to [`PING_TIP`][`PongProcess::PING_TIP`],
/// and answers them with pong messages with tip equals to [`PONG_TIP`][`PongProcess::PONG_TIP`]
/// and requested pong number.
///
/// Process will be stopped if there is some window of inactivity appears.
#[derive(Clone)]
pub struct PongProcess {
    /// Window of inactivity
    /// after that pong process will be stopped
    /// (in seconds).
    max_inactivity_window: f64,
}

impl PongProcess {
    /// Name of times, which fires after inactivity window.
    const TIMER_NAME: &'static str = "PONG_TIMER";
    /// Name of pong [message](Message) tip.
    pub const PONG_TIP: &'static str = "PONG";
    /// Name of ping [message](Message) tip.
    pub const PING_TIP: &'static str = super::ping::PingProcess::PING_TIP;

    /// Assistant function which sets timer with specified delay and name.
    fn set_timer(&self, ctx: &mut dyn Context) {
        ctx.set_timer(Self::TIMER_NAME.to_owned(), self.max_inactivity_window);
    }

    /// Creates new [`PongProcess`] with specified inactivity window (in seconds).
    pub fn new(max_inactivity_window: f64) -> Self {
        Self {
            max_inactivity_window,
        }
    }
}

impl Process for PongProcess {
    /// Called when system is started.
    /// Sets inactivity window timer.
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        self.set_timer(ctx);
        Ok(())
    }

    /// Called when timer is fired and stops the process.
    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
        assert_eq!(name, Self::TIMER_NAME);
        ctx.stop_process();
        Ok(())
    }

    /// Called when message is received.
    /// Checks if message tip is equal to [`PING_TIP`][`PongProcess::PING_TIP`],
    /// then sends pong message with tip equals to [`PONG_TIP`][`PongProcess::PONG_TIP`]
    /// and with requested pong number.
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
