//! Implementation of [`PingProcess`].

use crate::common::{context::Context, message::Message, process::Process};

/// [`PingProcess`] sends consecutive ping messages to the partner process with specified delay.
/// It waits for the consecutive pong responses, while the last pong response is not received.
#[derive(Clone)]
pub struct PingProcess {
    /// Specifies number of last received pong, initially equals to zero.
    last_pong: u32,
    /// Specifies delay between consecutive pings.
    delay: f64,
    /// Specifies name of partner process,
    /// which must send pongs in response to pings.
    /// For example, it can be [`PongProcess`][`super::pong::PongProcess`].
    partner: String,
    /// The number of pongs, which are needed to be received.
    need_cnt: u32,
    /// Indicates whether the process is started.
    is_started: bool,
    /// Indicates whether the process is stopped.
    is_stopped: bool,
    /// Specifies process speak ability.
    verbose: bool,
}

impl PingProcess {
    /// Number of actions, which process will do during ping.
    const ON_PING_ACTIONS: usize = 2;

    /// Number of actions, which process will do during start.
    pub const ON_START_ACTIONS: usize = Self::ON_PING_ACTIONS;

    /// Number of actions, which process will do during handling timer event.
    pub const ON_TIMER_ACTIONS: usize = Self::ON_PING_ACTIONS;

    /// Number of actions, which process will do during handling message event.
    pub const ON_MESSAGE_ACTIONS: usize = 0;

    /// Number of actions, which process will do during handling the last message event.
    pub const ON_LAST_MESSAGE_ACTIONS: usize = 2;

    /// Name of the timer, which fires if pong response do not received for a specified delay.
    pub const PING_TIMER: &'static str = "PING_TIMER";

    /// Name of ping [message](Message) tip.
    pub const PING_TIP: &'static str = "PING";

    /// Name of pong [message](Message) tip.
    pub const PONG_TIP: &'static str = super::pong::PongProcess::PONG_TIP;

    /// Performs ping action.
    ///
    /// # Panics
    /// - In case of message can not be created, which means incorrect implementation of [Message] class.
    fn ping(&self, ctx: &mut dyn Context) {
        let pong_request = self.last_pong + 1;

        let msg = Message::borrow_new(Self::PING_TIP, pong_request)
            .expect("Can not create ping message from u32");
        ctx.send_message(msg, self.partner.clone());
        ctx.set_timer(Self::PING_TIMER.to_owned(), self.delay);

        if self.verbose {
            println!("PingProcess: sent ping message with requested pong number={}.", pong_request);
        }
    }

    /// Returns last received pong number.
    pub fn get_last_pong(&self) -> u32 {
        self.last_pong
    }

    /// Allows to check whether the process is started.
    pub fn is_started(&self) -> bool {
        self.is_started
    }

    /// Allows to check whether the process is stopped.
    pub fn is_stopped(&self) -> bool {
        self.is_stopped
    }

    /// Creates new [`PingProcess`] with delay, partner process name
    /// and need count of pongs to receive before terminate.
    ///
    /// * `delay` must be greater than zero.
    /// * `need_cnt` must be greater than zero.
    pub fn new(delay: f64, partner: String, need_cnt: u32) -> Self {
        assert!(delay > 0.0);
        assert!(need_cnt > 0);

        Self {
            last_pong: 0,
            delay,
            partner,
            need_cnt,
            is_started: false,
            is_stopped: false,
            verbose: false,
        }
    }

    /// Creates new verbose [`PingProcess`] with delay, partner process name
    /// and need count of pongs to receive before terminate.
    /// 
    /// See [`PingProcess::new`] for details.
    pub fn new_verbose(delay: f64, partner: String, need_cnt: u32) -> Self {
        assert!(delay > 0.0);
        assert!(need_cnt > 0);

        Self {
            last_pong: 0,
            delay,
            partner,
            need_cnt,
            is_started: false,
            is_stopped: false,
            verbose: true,
        }
    }
}

impl Process for PingProcess {
    /// Called when system is started.
    /// Pings partner process.
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        if self.verbose {
            println!("PingProcess: stared.");
        }

        self.is_started = true;
        self.ping(ctx);
        Ok(())
    }

    /// Called when timer is fired.
    /// Retries to send ping message to the partner process.
    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
        assert_eq!(name, Self::PING_TIMER);
        self.ping(ctx);
        Ok(())
    }

    /// Called when message is received.
    /// If received message tip is equal to [`PONG_TIP`][Self::PONG_TIP],
    /// then process tripes to extract pong number from the message.
    /// If the number if equals to the expected pong number, then expected pong number is incremented.
    /// If after that last received pong number is equal to the expected number of pongs, then process is stopped.
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

        if self.verbose {
            println!("PingProcess: received pong response with sequence number={}.", pong_sequence_number);
        }

        if self.last_pong == self.need_cnt {
            ctx.cancel_timer(Self::PING_TIMER.into());
            ctx.stop_process();

            self.is_stopped = true;

            if self.verbose {
                println!("PingProcess: stopped.");
            }
        }

        Ok(())
    }
}
