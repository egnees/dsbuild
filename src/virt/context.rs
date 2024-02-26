//! Definition of virtual mode context.

use std::{cell::RefCell, future::Future, rc::Rc};

use crate::common::{message::Message, process::Address};
use dslab_async_mp::context::Context as DSLabContext;
use log::warn;

use super::node_manager::NodeManager;

/// Represents context in virtual mode.
/// Responsible for user-simulation interaction.
/// Serves as a proxy between user and underlying [DSLab MP simulation](https://github.com/osukhoroslov/dslab/tree/main/crates/dslab-mp),
/// uses corresponding [`DSLab MP context`][DSLabContext] methods.
#[derive(Clone)]
pub(crate) struct VirtualContext {
    pub process_address: Address,
    pub dslab_ctx: DSLabContext,
    pub node_manager: Rc<RefCell<NodeManager>>,
}

impl VirtualContext {
    /// Send local message.
    pub fn send_local(&self, message: Message) {
        self.dslab_ctx.send_local(message.into());
    }

    /// Set timer with specified name and delay.
    /// If such timer already exists, delay will be override.
    pub fn set_timer(&self, name: &str, delay: f64) {
        self.dslab_ctx.set_timer(name, delay);
    }

    /// Set timer with specified name and delay.
    /// If such timer already exists, nothing happens.
    pub fn set_timer_once(&self, name: &str, delay: f64) {
        self.dslab_ctx.set_timer_once(name, delay);
    }

    /// Cancel timer with specified name.
    pub fn cancel_timer(&self, name: &str) {
        self.dslab_ctx.cancel_timer(name);
    }

    /// Send message to specified address.
    pub fn send(&self, msg: Message, dst: Address) {
        match self.node_manager.borrow().get_full_process_name(&dst) {
            Ok(full_process_name) => {
                self.dslab_ctx.send(msg.into(), full_process_name);
            }
            Err(err) => {
                warn!("Message not sent: {}", err);
            }
        }
    }

    /// Spawn asynchronous activity.
    pub fn spawn(&self, future: impl Future<Output = ()>) {
        self.dslab_ctx.spawn(future);
    }
}
