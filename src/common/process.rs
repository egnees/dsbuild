//! Definition of trait [`Process`] and struct [`ProcessWrapper`].

use std::{
    ops::Deref,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use crate::common::{context::Context, message::Message};

/// Represents possible states of the user processes inside of the system.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ProcessState {
    /// Corresponds running process.
    Running,
    /// Corresponds to stopped process.
    Stopped,
}

/// Represents requirements for every user-defined process.
///
/// Every user-defined process must satisfy the following requirements:
/// - It must implement the [`Process`] trait.
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
/// To interact with system, process can use context [`Context`] object.
/// It allows [send messages][`Context::send`], [set timers][`Context::set_timer`], [work with file system][`Context::create_file`], etc.
pub trait Process: Send + Sync {
    /// Called when process starts interaction with system.
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String>;

    /// Called when previously set timer is fired.
    fn on_timer(&mut self, name: String, ctx: Context) -> Result<(), String>;

    /// Called when process receives message.
    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String>;
}

/// Represents wrapper around user-defined [`process`][crate::Process],
/// which returns to user when he passes [`process`][crate::Process] to [`real`][crate::RealNode] or [`virtual`][crate::Sim] system.
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

/// Represents guard for user-defined [`process`][`crate::Process`].
///
/// While user hold [`guard`][`ProcessGuard`] on the process, system can not get access to it.
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
            .expect("Can not read process, probably runtime panicked");

        ProcessGuard { inner: read_guard }
    }
}

/// Represents [`process`][`crate::Process`] address, which is used in
/// [`real node`][`crate::RealNode`] and [`virtual system`][`crate::Sim`]
///  to route [`network messages`][crate::Message].
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, Eq, serde::Deserialize, serde::Serialize)]
pub struct Address {
    /// Specifies host,
    /// which is used to deliver messages
    /// to the node instance through the network.
    pub host: String,

    /// Specifies port,
    /// which is used to deliver messages
    /// to the node instance
    /// through the network.
    pub port: u16,

    /// Specifies process name
    /// inside of the real node instance.
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

    /// Creates new address instance from string slices.
    pub fn new_ref(host: &str, port: u16, process_name: &str) -> Self {
        Self::new(host.to_owned(), port, process_name.to_owned())
    }

    /// Creates new node address instance with empty process name.
    pub(crate) fn new_node_address(host: String, port: u16) -> Self {
        Self::new(host, port, "".to_owned())
    }
}
