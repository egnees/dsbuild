//! Handle which can be used by process to interact with outer world.

use std::future::Future;

use crate::{real::context::RealContext, sim::context::VirtualContext, SendResult};

use super::{
    fs::{File, FsResult},
    message::{Message, Tag},
    process::Address,
};

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
enum ContextVariant {
    Real(RealContext),
    Virtual(VirtualContext),
}

////////////////////////////////////////////////////////////////////////////////

/// Handle which allows process to interact with external environment.
///
/// If implemented system is running within the simulation, then context allows to
/// interact with simulation. Else, system is running in the real environment and context
/// will interact with it. Using context process can send network messages to
/// other processes, set timers, manipulate with files and sent local messages to user.
/// Context is passed to the process by the system on every request to handle the external
/// world event.
///
/// For more details refer to [`Process`][crate::Process] documentation.
#[derive(Clone)]
pub struct Context {
    context_variant: ContextVariant,
}

impl Context {
    pub(crate) fn new_real(real_ctx: RealContext) -> Self {
        Self {
            context_variant: ContextVariant::Real(real_ctx),
        }
    }

    pub(crate) fn new_virt(virt_ctx: VirtualContext) -> Self {
        Self {
            context_variant: ContextVariant::Virtual(virt_ctx),
        }
    }

    ////////////////////////////////////////////////////////////////////////////////
    // Network
    ////////////////////////////////////////////////////////////////////////////////

    /// Allows to unreliable send network message to the specified process.
    ///
    /// It is not guaranteed the message will be delivered to destination.
    ///
    /// Simulation allows to configure delivery [probability][crate::Sim::set_network_drop_rate]
    /// and [delay][crate::Sim::set_network_delays].
    pub fn send(&self, msg: Message, dst: Address) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send(msg, dst),
            ContextVariant::Virtual(ctx) => ctx.send(msg, dst),
        }
    }

    /// Allows to reliable send network message to the specified process.
    ///
    /// On awaiting method will be blocked until the destination will receive
    /// message, send acknowledgment and it will be received by the sender's node.
    ///
    /// If acknowledgment was not received for `timeout` seconds,
    /// method will return with [`Timeout`][crate::SendError::Timeout].
    pub async fn send_with_ack(&self, msg: Message, dst: Address, timeout: f64) -> SendResult<()> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_with_ack(msg, dst, timeout).await,
            ContextVariant::Virtual(ctx) => ctx.send_with_ack(msg, dst, timeout).await,
        }
    }

    /// Allows to reliable send network message with specified [tag][Tag].
    ///
    /// On awaiting method will be blocked until the destination will not receive
    /// message and send acknowledgment. If acknowledgment was not received for `timeout` seconds,
    /// method will return with [`Timeout`][crate::SendError::Timeout].
    ///
    /// Note that receiver will not be explicitly
    /// notified about message was tagged, so it is on user to additionaly pass tag within
    /// the message if it is needed.
    pub async fn send_with_tag(
        &self,
        msg: Message,
        tag: Tag,
        to: Address,
        timeout: f64,
    ) -> SendResult<()> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_with_tag(msg, tag, to, timeout).await,
            ContextVariant::Virtual(ctx) => ctx.send_with_tag(msg, tag, to, timeout).await,
        }
    }

    /// Allows to reliable send network message with specified [tag][Tag] and wait for response.
    ///
    /// On awaiting method will be blocked until sender process will receive
    /// message with the same tag. On success, the received message will be returned.
    ///
    /// If message with provided tag will not be received in `timeout` seconds,
    /// method will return with [`Timeout`][crate::SendError::Timeout].
    ///
    /// See [`send_with_tag`][Context::send_with_tag] documentation for more details.
    pub async fn send_recv_with_tag(
        &self,
        msg: Message,
        tag: Tag,
        to: Address,
        timeout: f64,
    ) -> SendResult<Message> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_recv_with_tag(msg, tag, to, timeout).await,
            ContextVariant::Virtual(ctx) => ctx.send_recv_with_tag(msg, tag, to, timeout).await,
        }
    }

    ////////////////////////////////////////////////////////////////////////////////
    // Local
    ////////////////////////////////////////////////////////////////////////////////

    /// Spawn asynchronous activity.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.spawn(future),
            ContextVariant::Virtual(ctx) => ctx.spawn(future),
        }
    }

    /// Allows to stop the process.
    ///
    /// It is not guaranteed the process will be stopped immediately.
    pub fn stop(self) {
        match self.context_variant {
            ContextVariant::Real(ctx) => ctx.stop(),
            ContextVariant::Virtual(ctx) => ctx.stop(),
        }
    }

    /// Allows process to send message to user.
    ///
    /// User can read process local messages in simulation using provided
    /// [method][crate::Sim::read_local_messages].
    ///
    /// In real mode user can read
    /// process messages using [`IOProcessWrapper`][crate::real::io::IOProcessWrapper].
    pub fn send_local(&self, message: Message) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_local(message),
            ContextVariant::Virtual(ctx) => ctx.send_local(message),
        }
    }

    ////////////////////////////////////////////////////////////////////////////////
    // File system
    ////////////////////////////////////////////////////////////////////////////////

    /// Allows to create file.
    ///
    /// On success, created [file][File] will be returned.
    ///
    /// If there is no enough space to create file,
    /// [`BufferSizeExceed`][crate::FsError::BufferSizeExceed] error will be returned.
    pub async fn create_file<'a>(&'a self, name: &'a str) -> FsResult<File> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.create_file(name).await,
            ContextVariant::Virtual(ctx) => ctx.create_file(name).await,
        }
    }

    /// Allows to delete file.
    ///
    /// If [`NotFound`][`crate::FsError::NotFound`] error is returned,
    /// file was not found. Note, that the inverse is not true. If file not exists,
    /// its removal may fail for a numbers of reasons, such as not sufficient
    /// permissions.
    pub async fn delete_file<'a>(&'a self, name: &'a str) -> FsResult<()> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.delete_file(name).await,
            ContextVariant::Virtual(ctx) => ctx.delete_file(name).await,
        }
    }

    /// Allows to check if file exists.
    pub async fn file_exists<'a>(&'a self, name: &'a str) -> FsResult<bool> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.file_exists(name).await,
            ContextVariant::Virtual(ctx) => ctx.file_exists(name).await,
        }
    }

    /// Allows to open file with specified name.
    ///
    /// On success, [file][File] with provided name will be returned.
    ///
    /// If file does not exists, method will return with [`NotFound`][crate::FsError::NotFound]
    /// error.
    pub async fn open_file<'a>(&'a self, name: &'a str) -> FsResult<File> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.open_file(name).await,
            ContextVariant::Virtual(ctx) => ctx.open_file(name).await,
        }
    }

    ////////////////////////////////////////////////////////////////////////////////
    // Time
    ////////////////////////////////////////////////////////////////////////////////

    /// Allows to get current time on node in seconds.
    pub fn time(&self) -> f64 {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.time(),
            ContextVariant::Virtual(ctx) => ctx.time(),
        }
    }

    /// Set timer with specified name and delay.
    ///
    /// If timer with such name already exists, the delay will be override.
    pub fn set_timer(&self, name: &str, delay: f64) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.set_timer(name, delay),
            ContextVariant::Virtual(ctx) => ctx.set_timer(name, delay),
        }
    }

    /// Set timer with specified name and delay once.
    ///
    /// If such timer already exists, nothing happens.
    pub fn set_timer_once(&self, name: &str, delay: f64) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.set_timer_once(name, delay),
            ContextVariant::Virtual(ctx) => ctx.set_timer_once(name, delay),
        }
    }

    /// Cancel timer with specified name.
    pub fn cancel_timer(&self, name: &str) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.cancel_timer(name),
            ContextVariant::Virtual(ctx) => ctx.cancel_timer(name),
        }
    }
}
