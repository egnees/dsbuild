//! Definition of [`Context`].

use std::future::Future;

use dslab_async_mp::storage::MAX_BUFFER_SIZE;

use crate::real::context::RealContext;
use crate::virt::context::VirtualContext;

use super::{
    message::Message,
    process::Address,
    storage::{CreateFileError, ReadError, WriteError},
};

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
    pub async fn send_reliable(&self, msg: Message, dst: Address) -> Result<(), String> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_reliable(msg, dst).await,
            ContextVariant::Virtual(ctx) => ctx.send_reliable(msg, dst).await,
        }
    }

    /// Send reliable message to specified address.
    /// If message will not be delivered in specified timeout,
    /// error will be returned.
    pub async fn send_reliable_timeout(
        &self,
        msg: Message,
        dst: Address,
        timeout: f64,
    ) -> Result<(), String> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.send_reliable_timeout(msg, dst, timeout).await,
            ContextVariant::Virtual(ctx) => ctx.send_reliable_timeout(msg, dst, timeout).await,
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

    /// Async sleep for some time (sec.).
    pub async fn sleep(&self, duration: f64) {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.sleep(duration).await,
            ContextVariant::Virtual(ctx) => ctx.sleep(duration).await,
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
    pub async fn create_file(&self, name: &'static str) -> Result<(), CreateFileError> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.create_file(name).await,
            ContextVariant::Virtual(ctx) => ctx.create_file(name).await,
        }
    }

    /// Read file from specified offset into specified buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes read.
    ///
    /// # Panics
    ///    
    /// In case buf size exceeds [`MAX_BUFFER_SIZE`].
    pub async fn read(
        &self,
        file: &'static str,
        offset: usize,
        buf: &'static mut [u8],
    ) -> Result<usize, ReadError> {
        if buf.len() > MAX_BUFFER_SIZE {
            panic!(
                "buf size exceeds max buffer size: {} exceeds {}",
                buf.len(),
                MAX_BUFFER_SIZE
            );
        }

        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.read(file, offset, buf).await,
            ContextVariant::Virtual(ctx) => ctx.read(file, offset, buf).await,
        }
    }

    /// Append data to file.
    pub async fn append(&self, name: &'static str, data: &'static [u8]) -> Result<(), WriteError> {
        match &self.context_variant {
            ContextVariant::Real(ctx) => ctx.append(name, data).await,
            ContextVariant::Virtual(ctx) => ctx.append(name, data).await,
        }
    }
}
