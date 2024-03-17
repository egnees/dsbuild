//! Definition of [`Context`].

use std::future::Future;

use crate::real::context::RealContext;
use crate::virt::context::VirtualContext;

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
}
