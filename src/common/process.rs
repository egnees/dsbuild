//! Definition of trait [`Process`] and struct [`ProcessWrapper`].

use std::{
    ops::Deref,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use crate::common::{context::Context, message::Message};

////////////////////////////////////////////////////////////////////////////////

/// Represents possible states of the user processes inside of the system.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ProcessState {
    /// Corresponds running process.
    Running,
    /// Corresponds to stopped process.
    Stopped,
}

////////////////////////////////////////////////////////////////////////////////

/// Represents requirements for every user-defined process.
///
/// When process receives local message from user, timer is fired or
/// network message is received, the corresponding process callback will be called.
///
/// To interact with system, process can use passed [`Context`] object, which
/// represents proxy between process and external environment.
pub trait Process: Send + Sync {
    /// Called when process receives local message from user.
    ///
    /// See documentation of [`IOProcessWrapper`][crate::IOProcessWrapper] struct for real
    /// mode and [corresponding method][crate::Sim::send_local_message] of simulation for
    /// more details.
    fn on_local_message(&mut self, msg: Message, ctx: Context);

    /// Called when previously set timer is fired.
    ///
    /// See [corresponding method][Context::set_timer] of context for more details.
    fn on_timer(&mut self, name: String, ctx: Context);

    /// Called when process receives network message from other process.
    fn on_message(&mut self, msg: Message, from: Address, ctx: Context);
}

////////////////////////////////////////////////////////////////////////////////

/// Represents wrapper around user-defined [process][crate::Process] which provides
/// read access to it.
#[derive(Clone)]
pub struct ProcessWrapper<P: Process + 'static> {
    pub(crate) process_ref: Arc<RwLock<P>>,
    /// Address of the user-defined process.
    pub address: Address,
}

////////////////////////////////////////////////////////////////////////////////

/// Represents read access guard for user-defined [process][crate::Process].
///
/// Holding guard will prevent concurrent access to process by the system.
pub struct ProcessGuard<'a, P: Process + 'static> {
    pub(self) inner: RwLockReadGuard<'a, P>,
}

impl<P: Process + 'static> Deref for ProcessGuard<'_, P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<P: Process + 'static> ProcessWrapper<P> {
    /// Returns [guard][ProcessGuard] for read access to user-defined process.
    /// See [guard][ProcessGuard] documentation for more details.
    pub fn read(&self) -> ProcessGuard<'_, P> {
        let read_guard = self
            .process_ref
            .read()
            .expect("Can not read process, probably runtime panicked");

        ProcessGuard { inner: read_guard }
    }
}

////////////////////////////////////////////////////////////////////////////////

/// [Process][crate::Process] address, which is used to route
/// [network messages][crate::Message].
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, Eq, serde::Deserialize, serde::Serialize)]
pub struct Address {
    /// Specifies listen host of the owner node.
    pub host: String,

    /// Specifies listen port of the owner node.
    pub port: u16,

    /// Specifies process name within the owner node.
    pub process_name: String,
}

impl Address {
    /// Creates new address instance.
    pub fn new(host: String, port: u16, process_name: String) -> Self {
        Self {
            host,
            port,
            process_name,
        }
    }

    /// Creates new address instance from string slice.
    pub fn new_ref(host: &str, port: u16, process_name: &str) -> Self {
        Self::new(host.to_owned(), port, process_name.to_owned())
    }

    /// Creates new node address instance with empty process name.
    pub(crate) fn new_node_address(host: String, port: u16) -> Self {
        Self::new(host, port, String::new())
    }
}
