//! Handle which can be used by process to interact with outer world.

use std::future::Future;

use dslab_async_mp::network::result::SendResult;

use crate::{real::context::RealContext, sim::context::VirtualContext, storage::StorageResult};

use super::{fs::File, message::Message, process::Address, tag::Tag};

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
enum ContextVariant {
    Real(RealContext),
    Virtual(VirtualContext),
}

////////////////////////////////////////////////////////////////////////////////

/// Handle which allows process to interact with outer world. For example,
/// using context process can send network messages to other processes,
/// set timers, manipulate with files and sent local messages to user.
/// Context is passed to the process by the system on every request to handle
/// the outer world event.
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
    /// Network
    ////////////////////////////////////////////////////////////////////////////////

    /// Allows to send network message to the specified process.
    /// It is not guaranteed the message will be delivered to destination.
    /// Simulation allows to configure delivery probability and delay.
    pub fn send(&self, msg: Message, dst: Address) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send(msg, dst),
            ContextVariant::Virtual(ctx) => ctx.send(msg, dst),
        }
    }

    /// Allows to reliable send network message to the specified process.
    /// On awaiting method will be blocked until the destination will not receive
    /// the message and send acknowledgment. If acknowledgment was not received for `timeout` seconds,
    /// method will return with timeout error.
    pub async fn send_with_ack(&self, msg: Message, dst: Address, timeout: f64) -> SendResult<()> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_with_ack(msg, dst, timeout).await,
            ContextVariant::Virtual(ctx) => ctx.send_with_ack(msg, dst, timeout).await,
        }
    }

    /// Allows to reliable send network message with specified [tag][Tag].
    /// On awaiting method will be blocked until the destination will not receive
    /// the message and send acknowledgment. If acknowledgment was not received for `timeout` seconds,
    /// method will return with timeout error.
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

    /// Allows to reliable send network message with specified [tag][Tag].
    /// On awaiting method will be blocked until process will not receive
    /// message with the same tag. On success, the received message will be returned.
    /// If message with provided tag will not be received in `timeout` seconds,
    /// method will retunr with timeout error.
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
    /// Local
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
    /// It is not guaranteed the process will be stopped immediately.
    pub fn stop(self) {
        match self.context_variant {
            ContextVariant::Real(ctx) => ctx.stop(),
            ContextVariant::Virtual(ctx) => ctx.stop(),
        }
    }

    /// Allows process to send message to user.
    /// User can read particular process messages in simulation using provided
    /// [method][crate::Sim::read_local_messages]. In real mode user can read
    /// process messages using [IOProcessWrapper][crate::real::io::IOProcessWrapper].
    pub fn send_local(&self, message: Message) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_local(message),
            ContextVariant::Virtual(ctx) => ctx.send_local(message),
        }
    }

    ////////////////////////////////////////////////////////////////////////////////
    /// File system
    ////////////////////////////////////////////////////////////////////////////////

    /// Allows to crate file. On success, created [file][File] will be returned.
    pub async fn create_file<'a>(&'a self, name: &'a str) -> StorageResult<File> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.create_file(name).await,
            ContextVariant::Virtual(ctx) => ctx.create_file(name).await,
        }
    }

    /// Allows to check if file exists.
    pub async fn file_exists<'a>(&'a self, name: &'a str) -> StorageResult<bool> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.file_exists(name).await,
            ContextVariant::Virtual(ctx) => ctx.file_exists(name).await,
        }
    }

    /// Allows to opens file with specified name.
    /// On success, [file][File] with provided name will be returned.
    /// If file does not exists, method will return with the error.
    pub async fn open_file<'a>(&'a self, name: &'a str) -> StorageResult<File> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.open_file(name).await,
            ContextVariant::Virtual(ctx) => ctx.open_file(name).await,
        }
    }

    ////////////////////////////////////////////////////////////////////////////////
    /// Time
    ////////////////////////////////////////////////////////////////////////////////

    /// Allows to get current system time process' node in seconds.
    pub fn time(&self) -> f64 {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.time(),
            ContextVariant::Virtual(ctx) => ctx.time(),
        }
    }

    /// Allows to set timer with specified name and delay.
    /// If timer with such name already exists, the delay will be override.
    pub fn set_timer(&self, name: &str, delay: f64) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.set_timer(name, delay),
            ContextVariant::Virtual(ctx) => ctx.set_timer(name, delay),
        }
    }

    /// Set timer with specified name and delay.
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
