//! Definition of user process actions.

use crate::common::message::Message;

/// Specifies the behaviour of timer set
/// in the presence of existing active timer with this name.
#[derive(Clone, Debug)]
pub enum TimerBehavior {
    /// Do not override the existing timer delay.
    SetOnce,
    /// Override the existing timer delay.
    OverrideExisting,
}

/// Specifies system policy on stopping user process.
#[derive(Clone, Debug)]
pub enum StopPolicy {
    /// Stop immediately and ignore all pending events and futher events.
    Immediately,
}

/// Specifies actions which can be caused by user process.
#[derive(Clone, Debug)]
pub enum ProcessAction {
    /// Specifies message sent action.
    MessageSent {
        msg: Message,
        from: String,
        to: String,
    },
    /// Specifies timer establishment action.
    TimerSet {
        process_name: String,
        timer_name: String,
        delay: f64,
        behavior: TimerBehavior,
    },
    /// Specifies timer canceling action.
    TimerCancelled {
        process_name: String,
        timer_name: String,
    },
    /// Specifies user request to stop the process.
    ProcessStopped {
        process_name: String,
        policy: StopPolicy,
    },
}
