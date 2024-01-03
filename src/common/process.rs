//! Definition of trait Process.

use std::{
    ops::Deref,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use dyn_clone::DynClone;

use crate::common::{context::Context, message::Message};

/// Represents possible states of the process
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ProcessState {
    Inited,
    Running,
    Stopped,
}

/// Represents requirements for every user-defined Process struct.
pub trait Process: DynClone {
    /// Called in the beginning of the interaction with system.
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String>;

    /// Called on timer fired.
    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String>;

    /// Called on message received.
    fn on_message(
        &mut self,
        msg: Message,
        from: String,
        ctx: &mut dyn Context,
    ) -> Result<(), String>;
}

#[derive(Clone)]
pub struct ProcessWrapper<P: Process + 'static> {
    pub(crate) process_ref: Arc<RwLock<P>>,
}

pub struct ProcessGuard<'a, P: Process + 'static> {
    pub(self) inner: RwLockReadGuard<'a, P>,
}

impl<P: Process + 'static> Deref for ProcessGuard<'_, P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<P: Process + 'static> ProcessWrapper<P> {
    pub fn read(&self) -> ProcessGuard<'_, P> {
        let read_guard = self
            .process_ref
            .read()
            .expect("Can not read process, probably runtime has been panicked");

        ProcessGuard { inner: read_guard }
    }
}
