//! Definition of events inside of the system in real mode.

use crate::common::{message::Message, process::Address};

/// Specifies events appearing in the system in real mode.
#[derive(Clone, PartialEq, Debug)]
pub enum Event {
    /// Specifies event of timer firing.
    TimerFired {
        process_name: String,
        timer_name: String,
    },
    /// Specifies event of receiving message.
    MessageReceived {
        msg: Message,
        /// Address of the message sender.
        from: Address,
        /// Name of message-receiver.
        to: String,
    },
    /// Specifies event of system start.
    SystemStarted {},
}
