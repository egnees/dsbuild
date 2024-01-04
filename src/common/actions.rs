//! Definition of user process [actions](`ProcessAction`).

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
    /// Stop immediately and ignore all pending and futher events.
    Immediately,
}

/// Specifies actions which can be caused by user process.
#[derive(Clone, Debug)]
pub enum ProcessAction {
    /// Specifies message sent action.
    MessageSent {
        /// Message which was sent.
        msg: Message,
        /// Name of process-sender of message.
        from: String,
        /// Name of process-receiver of message.
        to: String,
    },
    /// Specifies timer establishment action.
    TimerSet {
        /// Name of process which set the timer.
        process_name: String,
        /// Name of timer.
        timer_name: String,
        /// Delay of timer in seconds.
        delay: f64,
        /// Specifies behaviour of timer in case of
        /// such timer already exists.
        behavior: TimerBehavior,
    },
    /// Specifies timer canceling action.
    TimerCancelled {
        /// Name of process which cancelled the timer.
        process_name: String,
        /// Name of timer.
        timer_name: String,
    },
    /// Specifies user request to stop the process.
    ProcessStopped {
        /// Name of process, requested to stop.
        process_name: String,
        /// Specifies policy on stopping process.
        policy: StopPolicy,
    },
}
