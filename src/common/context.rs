//! Definition of trait Context.

/// Represents proxy, which provides process-system interaction. 
pub trait Context {
    /// Sets a timer without overriding existing one delay. 
    fn set_timer_once(&mut self, name: String, delay: f64);

    /// Sets a timer with overriding existing one delay.
    fn set_timer(&mut self, name: String, delay: f64);

    /// Cancel timer with certain name.
    /// If there is no such timer, does nothing.
    fn cancel_timer(&mut self, name: String);

    /// Stop process.
    /// If immediately flag is on, then all pending and further actions will be ignored.
    /// Else, process will be stopped only after handling all pending events.
    fn stop_process(&mut self, immediately: bool);
}

