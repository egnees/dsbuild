//! Definition of trait [`Process`] and struct [`ProcessWrapper`].

use std::{
    ops::Deref,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use dyn_clone::DynClone;

use crate::common::{context::Context, message::Message};

/// Represents possible states of the user processes inside of the system.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ProcessState {
    /// Corresponds to initial state of the process.
    Initialized,
    /// Corresponds to state when the process is running.
    Running,
    /// Corresponds to state when the process is stopped.
    Stopped,
}

/// Represents requirements for every user-defined process.
///
/// Every user-defined process must satisfy the following requirements:
/// - It must implement the [`Process`] trait.
/// - It must implement the [`Clone`] trait.
///
/// Ideologically every process must be created by the user,
/// and after that passed to the system (real or virtual), which will own the process.
/// But technically process got static lifetime when being passed to the system,
/// so system not holds the process.
/// However, only system has write access to the process.
///
/// The reason for such behavior is that process can be used by user after the system is dropped,
/// so system can not hold the process.
/// So, for system every process' lifetime is static.
///
/// To interact with system, process can use context object, which implements [`Context`] trait.
/// It allows to send messages, set timers, etc.
pub trait Process: DynClone {
    /// Called when process starts interaction with system.
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String>;

    /// Called when previously set timer is fired.
    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String>;

    /// Called when process receives message.
    fn on_message(
        &mut self,
        msg: Message,
        from: String,
        ctx: &mut dyn Context,
    ) -> Result<(), String>;
}

/// Represents wrapper around user-defined process,
/// which returns to user when he passes process to system.
///
/// Wrapper holds reference to user-defined process, which implements [`Process`] trait,
/// and allows user to get read access to it.
///
/// User process inside of [`ProcessWrapper`] is protected with [lock][`RwLock`],
/// which prevents concurrent read-write access to the process.
/// It allows multiple readers or only one writer in the same time.
#[derive(Clone)]
pub struct ProcessWrapper<P: Process + 'static> {
    pub(crate) process_ref: Arc<RwLock<P>>,
}

/// Represents guard for user-defined process.
/// While user hold [`ProcessGuard`] on the process, system can not get access to it.
/// In this case system thread will be blocked until guard won't be dropped.
///
/// Technically, for now both real and virtual systems works with process in the same thread as user does,
/// but as process with static lifetime can not be owned by system[^note], [Rust](https://www.rust-lang.org/) requires it to be guarded with [lock][`RwLock`].
///
/// [^note]: in [Rust](https://www.rust-lang.org/) every variable with static lifetime must be guarded with some lock like [`std::sync::Mutex`] or [`std::sync::RwLock`].
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
    /// Returns guard for read access to user-defined process.
    /// Holding guard will prevent concurrent access to user-defined process.
    ///
    /// Is is allowed to have multiple guards on the same process in the same time,
    /// because [`ProcessWrapper`] gives only read access to the process.
    /// Note what having multiple readers in the same time not violates [Rust](https://www.rust-lang.org/) memory management rules.
    ///
    /// # Panics
    ///
    /// - If panicked thread in which runtime was launched. For more information see [`std::sync::RwLock#poisoning`] documentation.
    pub fn read(&self) -> ProcessGuard<'_, P> {
        let read_guard = self
            .process_ref
            .read()
            .expect("Can not read process, probably runtime has been panicked");

        ProcessGuard { inner: read_guard }
    }
}
