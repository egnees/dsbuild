//! Definition of trait [`Context`].

use std::future::Future;

use crate::{real::context::RealContext, virt::context::VirtualContext};

use super::{message::Message, process::Address};

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

    pub fn send_local(&self, message: Message) {
        match self.context_variant {
            ContextVariant::Real(_) => todo!(),
            ContextVariant::Virtual(ctx) => ctx.send_local(message),
        }
    }

    pub fn set_timer(&self, name: &str, delay: f64) {
        match self.context_variant {
            ContextVariant::Real(_) => todo!(),
            ContextVariant::Virtual(ctx) => ctx.set_timer(name, delay),
        }
    }

    pub fn set_timer_once(&self, name: &str, delay: f64) {
        match self.context_variant {
            ContextVariant::Real(_) => todo!(),
            ContextVariant::Virtual(ctx) => ctx.set_timer_once(name, delay),
        }
    }

    pub fn cancel_timer(&self, name: &str) {
        match self.context_variant {
            ContextVariant::Real(_) => todo!(),
            ContextVariant::Virtual(ctx) => ctx.cancel_timer(name),
        }
    }

    pub fn send(&self, msg: Message, dst: Address) {
        match self.context_variant {
            ContextVariant::Real(_) => todo!(),
            ContextVariant::Virtual(ctx) => ctx.send(msg, dst),
        }
    }

    pub fn spawn(&self, future: impl Future<Output = ()>) {
        match self.context_variant {
            ContextVariant::Real(_) => todo!(),
            ContextVariant::Virtual(ctx) => ctx.spawn(future),
        }
    }
}
