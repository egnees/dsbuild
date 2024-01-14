//! Definition of [`process wrapper`][`VirtualProcessWrapper`] which is used in the virtual system.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, RwLock},
};

use log::warn;

use crate::common::{
    actions::{ProcessAction, StopPolicy, TimerBehavior},
    process::{Address, Process, ProcessState},
};

use dslab_mp::{
    context::Context as SimulationContext, message::Message as SimulationMessage,
    process::Process as SimulationProcess,
};

use super::{node_manager::NodeManager, virtual_context::VirtualContext};

/// Represents virtual process wrapper, which is to be passed to the [`DSLab MP`](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html).
#[derive(Clone)]
pub struct VirtualProcessWrapper<P: Process + Clone + 'static> {
    process_address: Address,
    user_process: Arc<RwLock<P>>,
    process_state: ProcessState,
    node_manager: Rc<RefCell<NodeManager>>,
}

impl<P: Process + Clone + 'static> VirtualProcessWrapper<P> {
    /// Create new virtual process wrapper.
    pub fn new(
        process_address: Address,
        process_impl: Arc<RwLock<P>>,
        node_manager: Rc<RefCell<NodeManager>>,
    ) -> Self {
        // let full_process_name = node_manager
        //     .borrow()
        //     .get_full_process_name(&process_address)
        //     .expect("Implementation error: can not get full process name");

        Self {
            process_address,
            user_process: process_impl,
            process_state: ProcessState::Initialized,
            node_manager,
        }
    }

    /// Create virtual context, which matches to the virtual process wrapper.
    fn create_context(&self) -> VirtualContext {
        VirtualContext {
            process_address: self.process_address.clone(),
            actions: Vec::new(),
        }
    }

    /// Handle process actions and transform them to the
    /// [`DSLab MP`](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html) actions.
    fn handle_process_actions(&mut self, actions: Vec<ProcessAction>, ctx: &mut SimulationContext) {
        for action in actions {
            if self.process_state == ProcessState::Stopped {
                break;
            }
            match action {
                ProcessAction::MessageSent { msg, from: _, to } => {
                    // Try to resolve full process name by its address.
                    match self.node_manager.borrow().get_full_process_name(&to) {
                        Ok(full_process_name) => {
                            ctx.send(msg.into(), full_process_name);
                        }
                        Err(err) => {
                            warn!("Message not sent: {}", err);
                        }
                    }
                }

                ProcessAction::ProcessStopped {
                    process_name: _,
                    policy,
                } => match policy {
                    StopPolicy::Immediately => {
                        assert!(self.process_state != ProcessState::Initialized);
                        self.process_state = ProcessState::Stopped;
                    }
                },

                ProcessAction::TimerSet {
                    process_name: _,
                    timer_name,
                    delay,
                    behavior,
                } => match behavior {
                    TimerBehavior::SetOnce => ctx.set_timer_once(timer_name.as_str(), delay),
                    TimerBehavior::OverrideExisting => ctx.set_timer(timer_name.as_str(), delay),
                },

                ProcessAction::TimerCancelled {
                    process_name: _,
                    timer_name,
                } => ctx.cancel_timer(timer_name.as_str()),
            }
        }
    }
}

/// Implementation of [`DSLab MP`](https://osukhoroslov.github.io/dslab_mp/index.html) process trait.
impl<P: Process + Clone + 'static> SimulationProcess for VirtualProcessWrapper<P> {
    fn on_message(
        &mut self,
        msg: SimulationMessage,
        from: String,
        ctx: &mut SimulationContext,
    ) -> Result<(), String> {
        if self.process_state == ProcessState::Stopped {
            return Ok(());
        }

        // Create virtual context to pass it into dslab process.
        let mut virt_ctx = self.create_context();

        // Get process address by it's full name.
        let from_address = self
            .node_manager
            .borrow()
            .get_process_address(&from)
            .expect("Incorrect implementation: received message from not registered process.");

        // Callback dslab process on message method.
        let result = self
            .user_process
            .write()
            .expect("Can not write in process, probably datarace appeared")
            .on_message(msg.into(), from_address, &mut virt_ctx);

        // handle process actions.
        self.handle_process_actions(virt_ctx.actions, ctx);

        result
    }

    fn on_local_message(
        &mut self,
        msg: SimulationMessage,
        ctx: &mut SimulationContext,
    ) -> Result<(), String> {
        assert_eq!(msg.tip, "START");

        assert_eq!(self.process_state, ProcessState::Initialized);

        self.process_state = ProcessState::Running;

        let mut virt_ctx = self.create_context();

        let result = self
            .user_process
            .write()
            .expect("Can not write in process, probably datarace appeared")
            .on_start(&mut virt_ctx);

        self.handle_process_actions(virt_ctx.actions, ctx);

        result
    }

    fn on_timer(&mut self, timer: String, ctx: &mut SimulationContext) -> Result<(), String> {
        if self.process_state == ProcessState::Stopped {
            return Ok(());
        }

        let mut virt_ctx = self.create_context();

        let result = self
            .user_process
            .write()
            .expect("Can not write in process, probably datarace appeared")
            .on_timer(timer, &mut virt_ctx);

        self.handle_process_actions(virt_ctx.actions, ctx);

        result
    }
}
