//! Definition of trait Process.

use crate::common::{context::Context, message::Message};

/// Represents possible states of the process
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ProcessState {
    Inited,
    Running,
    Stopping,
    Stopped
}

/// Represents requirements for every user-defined Process struct.
pub trait Process: Clone {
    /// Called in the beginning of the interaction with system.
    fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String>;

    /// Called on timer fired.
    fn on_timer(&mut self, name: String, ctx: &mut impl Context) -> Result<(), String>;

    /// Called on message received.
    fn on_message(&mut self, msg: Message, from: String, ctx: &mut impl Context) -> Result<(), String>; 
}