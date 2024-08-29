//! Definition of [`Context`].

use std::future::Future;

use dslab_async_mp::network::result::SendResult;

use crate::sim::context::VirtualContext;
use crate::{real::context::RealContext, storage::StorageResult};

use super::file::File;
use super::tag::Tag;
use super::{message::Message, process::Address};

/// Represents enum of two context variants - real and virtual.
#[derive(Clone)]
enum ContextVariant {
    Real(RealContext),
    Virtual(VirtualContext),
}
/// Represents proxy, which provides process-system interaction.
#[derive(Clone)]
pub struct Context {
    context_variant: ContextVariant,
}

impl Context {
    /// Create new real context.
    pub(crate) fn new_real(real_ctx: RealContext) -> Self {
        Self {
            context_variant: ContextVariant::Real(real_ctx),
        }
    }

    /// Create new virtual context.
    pub(crate) fn new_virt(virt_ctx: VirtualContext) -> Self {
        Self {
            context_variant: ContextVariant::Virtual(virt_ctx),
        }
    }

    /// Send local message.
    pub fn send_local(&self, message: Message) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_local(message),
            ContextVariant::Virtual(ctx) => ctx.send_local(message),
        }
    }

    /// Set timer with specified name and delay.
    /// If such timer already exists, delay will be override.
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

    /// Send message to specified address.
    pub fn send(&self, msg: Message, dst: Address) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send(msg, dst),
            ContextVariant::Virtual(ctx) => ctx.send(msg, dst),
        }
    }

    /// Send reliable message to specified address.
    /// If message will not be delivered in specified timeout,
    /// error will be returned.
    pub async fn send_with_ack(&self, msg: Message, dst: Address, timeout: f64) -> SendResult<()> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_with_ack(msg, dst, timeout).await,
            ContextVariant::Virtual(ctx) => ctx.send_with_ack(msg, dst, timeout).await,
        }
    }

    /// Allows to send message with specified tag reliable.
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

    /// Send reliable message to specified address
    /// and await message sent via [`Context::send_with_tag`] with specified tag
    /// from any process.
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

    /// Stop the process.
    pub fn stop(self) {
        match self.context_variant {
            ContextVariant::Real(ctx) => ctx.stop(),
            ContextVariant::Virtual(ctx) => ctx.stop(),
        }
    }

    /// Create file.
    pub async fn create_file<'a>(&'a self, name: &'a str) -> StorageResult<File> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.create_file(name).await,
            ContextVariant::Virtual(ctx) => ctx.create_file(name).await,
        }
    }

    /// Check if file exists.
    pub async fn file_exists<'a>(&'a self, name: &'a str) -> StorageResult<bool> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.file_exists(name).await,
            ContextVariant::Virtual(ctx) => ctx.file_exists(name).await,
        }
    }

    /// Open file with specified name.
    pub async fn open_file<'a>(&'a self, name: &'a str) -> StorageResult<File> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.open_file(name).await,
            ContextVariant::Virtual(ctx) => ctx.open_file(name).await,
        }
    }

    /// Returns current system time.
    pub fn time(&self) -> f64 {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.time(),
            ContextVariant::Virtual(ctx) => ctx.time(),
        }
    }
}
