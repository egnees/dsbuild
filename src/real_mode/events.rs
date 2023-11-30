//! Definion of events inside of the system in real mode.

use crate::common::message::Message;

/// Specifies events appearing in the system in real mode.
#[derive(Clone, PartialEq, Debug)]
pub enum Event {
    /// Specifies event of timer firing.
    TimerFired {
        name: String,
    },
    /// Specifies event of receiving message.
    MessageReceived {
        msg: Message,
        from: String,
    },
    /// Specifies event of starting system.
    SystemStarted {}
}