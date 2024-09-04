//! Definition of [`process wrapper`][`VirtualProcessWrapper`] which is used in the virtual system.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, RwLock},
};

use dslab_async_mp::{
    network::message::Message as DSLabMessage,
    process::{context::Context as DSLabContext, process::Process as DSLabProcess},
};

use crate::{
    common::process::{Process, ProcessState},
    Context,
};

use super::{context::VirtualContext, node::NodeManager};

/// Represents virtual process wrapper,
/// which is to be passed to the [`DSLab MP`](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html).
pub struct VirtualProcessWrapper<P: Process + 'static> {
    user_process: Arc<RwLock<P>>,
    process_state: ProcessState,
    node_manager: Rc<RefCell<NodeManager>>,
}

impl<P: Process + 'static> VirtualProcessWrapper<P> {
    /// Create new virtual process wrapper.
    pub fn new(process_impl: Arc<RwLock<P>>, node_manager: Rc<RefCell<NodeManager>>) -> Self {
        Self {
            user_process: process_impl,
            process_state: ProcessState::Running,
            node_manager,
        }
    }

    /// Create virtual context, which matches to the virtual process wrapper.
    fn create_context(&self, dslab_ctx: DSLabContext) -> VirtualContext {
        VirtualContext {
            dslab_ctx,
            node_manager: self.node_manager.clone(),
        }
    }
}

/// Implementation of [`DSLab MP`](https://osukhoroslov.github.io/dslab_mp/index.html) process trait.
impl<P: Process + 'static> DSLabProcess for VirtualProcessWrapper<P> {
    fn on_message(
        &mut self,
        msg: DSLabMessage,
        from: String,
        ctx: DSLabContext,
    ) -> Result<(), String> {
        if self.process_state == ProcessState::Stopped {
            return Ok(());
        }

        // Get process address by it's full name.
        let from_address = self
            .node_manager
            .borrow()
            .get_process_address(&from)
            .expect("Incorrect implementation: received message from not registered process.");

        // Create virtual context to pass it into dslab process.
        let virt_ctx = self.create_context(ctx);

        // Callback dslab process on message method.
        self.user_process
            .write()
            .expect("Can not write in process, probably datarace appeared")
            .on_message(msg.into(), from_address, Context::new_virt(virt_ctx))
    }

    fn on_local_message(&mut self, msg: DSLabMessage, ctx: DSLabContext) -> Result<(), String> {
        let virt_ctx = self.create_context(ctx);

        self.user_process
            .write()
            .expect("Can not write in process, probably datarace appeared")
            .on_local_message(msg.into(), Context::new_virt(virt_ctx))
    }

    fn on_timer(&mut self, timer: String, ctx: DSLabContext) -> Result<(), String> {
        if self.process_state == ProcessState::Stopped {
            return Ok(());
        }

        let virt_ctx = self.create_context(ctx);

        self.user_process
            .write()
            .expect("Can not write in process, probably datarace appeared")
            .on_timer(timer, Context::new_virt(virt_ctx))
    }
}
