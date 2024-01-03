//! Ping Process (Pinger) implementation.

use crate::common::{context::Context, message::Message, process::Process};

// PingProcess must send consecutive ping messages to the other one process with specified delay.
// Also, it waits for the consecutive pong answers.
#[derive(Clone)]
pub struct PingProcess {
    last_pong: u32,
    // delay in seconds
    delay: f64,
    // pings receiver
    partner: String,
    // need pongs count
    need_cnt: u32,
    // enable flag when on start method is called
    is_started: bool,
    // enable flag when process received the last expected pong
    is_stoped: bool,
}

impl PingProcess {
    const ON_PING_ACTIONS: usize = 2;

    pub const ON_START_ACTIONS: usize = Self::ON_PING_ACTIONS;
    pub const ON_TIMER_ACTIONS: usize = Self::ON_PING_ACTIONS;
    pub const ON_MESSAGE_ACTIONS: usize = 0;
    pub const ON_LAST_MESSAGE_ACTIONS: usize = 2;

    pub const PING_TIMER: &'static str = "PING_TIMER";
    pub const PING_TIP: &'static str = "PING";
    pub const PONG_TIP: &'static str = "PONG";

    fn ping(&self, ctx: &mut dyn Context) {
        let msg = Message::borrow_new(Self::PING_TIP, self.last_pong + 1)
            .expect("Can not create ping message");
        ctx.send_message(msg, self.partner.clone());
        ctx.set_timer(Self::PING_TIMER.to_owned(), self.delay);
    }

    pub fn get_last_pong(&self) -> u32 {
        self.last_pong
    }

    pub fn is_started(&self) -> bool {
        self.is_started
    }

    pub fn is_stoped(&self) -> bool {
        self.is_stoped
    }

    pub fn new(delay: f64, partner: String, need_cnt: u32) -> Self {
        assert!(delay > 0.0);
        Self {
            last_pong: 0,
            delay,
            partner,
            need_cnt,
            is_started: false,
            is_stoped: false,
        }
    }
}

impl Process for PingProcess {
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        self.is_started = true;
        self.ping(ctx);
        Ok(())
    }

    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
        assert_eq!(name, Self::PING_TIMER);
        self.ping(ctx);
        Ok(())
    }

    fn on_message(
        &mut self,
        msg: Message,
        from: String,
        ctx: &mut dyn Context,
    ) -> Result<(), String> {
        assert_eq!(self.partner, from);
        assert_eq!(msg.get_tip(), Self::PONG_TIP);

        let pong_sequence_number = msg.get_data::<u32>().expect("Can not get message data");
        if pong_sequence_number == self.last_pong + 1 {
            self.last_pong += 1;
        }

        if self.last_pong == self.need_cnt {
            ctx.cancel_timer(Self::PING_TIMER.into());
            ctx.stop_process();

            self.is_stoped = true;
        }

        Ok(())
    }
}
