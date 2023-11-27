//! Definition of user process actions.

use crate::common::message::Message;

/// Specifies the behaviour of timer set in the presence of existing active timer with this name.
#[derive(Clone, PartialEq, Debug)]
pub enum TimerBehavior {
    /// Do not override the existing timer delay.
    SetOnce,
    /// Override the existing timer delay.
    OverrideExisting,
}

/// Specifies system policy on stopping user process.
#[derive(Clone, PartialEq, Debug)]
pub enum StopPolicy {
    /// Stop after handling all pending events.
    Defer,
    /// Stop immediately and ignore all pending events and futher events.
    Immediately,
}

#[derive(Clone, Debug)]
pub enum ProcessActions {
    MessageSent {
        msg: Message,
        src: String,
        dst: String,
    },
    TimerSet {
        name: String,
        delay: f64,
        behavior: TimerBehavior,
    },
    TimerCancelled {
        name: String,
    },
    ProcessStopped {
        policy: StopPolicy,
    },
}