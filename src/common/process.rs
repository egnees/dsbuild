//! Definition of trait Process.

use crate::common::context::Context;

/// Represents requirements for every user-defined Process struct.
pub trait Process {
    /// Called in the beginning of the interaction with system.
    fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String>;

    /// Called on timer fired.
    fn on_timer(&mut self, name: String, ctx: &mut impl Context) -> Result<(), String>;
}