//! Definitions, which are used by timer manager and its users.

/// Specifies request of setting timer.
#[derive(Debug, Clone)]
pub struct SetTimerRequest {
    /// Process, which created request.
    pub process: String,
    /// Name of timer.
    pub timer_name: String,
    /// Timer delay in seconds.
    pub delay: f64,
}

/// Specifies event of timer triggering.
#[derive(Debug, Clone)]
pub struct TimerFiredEvent {
    /// Name of process which set timer.
    pub process: String,
    /// Name of timer.
    pub timer_name: String,
}
