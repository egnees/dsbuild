//! Definition of trait [`Context`].

use dyn_clone::DynClone;

use super::{message::Message, process::Address};

/// Represents proxy, which provides process-system interaction.
pub trait Context: DynClone {
    /// Sets a timer without overriding delay of existing one.
    fn set_timer_once(&mut self, name: String, delay: f64);

    /// Sets a timer with overriding delay of existing one.
    fn set_timer(&mut self, name: String, delay: f64);

    /// Cancel timer with certain name.
    /// If there is no such timer, does nothing.
    fn cancel_timer(&mut self, name: String);

    /// Send message to another process.
    fn send_message(&mut self, msg: Message, to: Address);

    /// Stop the process.
    fn stop_process(&mut self);
}
